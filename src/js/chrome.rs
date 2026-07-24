use std::time::Duration;

use anyhow::{anyhow, Result};
use futures::StreamExt;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};
use url::Url;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;

use super::engine::{JsEngine, JsResult};

/// Commands sent to the Chrome actor task.
enum Cmd {
    /// Navigate the browser to `url` and reply with the rendered HTML and the
    /// final URL (after any redirects / JS-driven navigation).
    Navigate {
        url: String,
        reply: oneshot::Sender<Result<(String, String)>>,
    },
}

/// Chrome headless backend via the Chrome DevTools Protocol.
///
/// Requires `google-chrome`, `chromium`, or `chromium-browser` on PATH.
///
/// `JsEngine::execute` is synchronous, but `chromiumoxide` is fully async. The
/// browser therefore lives in a dedicated actor task spawned on the tokio
/// runtime; `execute` bridges into it with `tokio::task::block_in_place` +
/// `Handle::block_on` (valid because `clibrowser` runs on a multi-threaded
/// runtime). All non-`Send`/non-`Sync` browser state stays inside the actor;
/// only channels cross the boundary, so `ChromeEngine` is `Send + Sync`.
///
/// v1 limitations (by design, documented for future work):
/// - Chrome manages its own cookies; they are not synced with the reqwest
///   `CookieStore`. GET navigation with this backend skips reqwest entirely.
/// - No `console.*` capture.
/// - POST form submission still uses the reqwest path; the response is not
///   re-rendered by Chrome (a Chrome re-navigation would turn the POST into a
///   GET).
pub struct ChromeEngine {
    handle: Handle,
    sender: std::sync::Mutex<Option<mpsc::Sender<Cmd>>>,
}

impl ChromeEngine {
    pub fn new() -> Self {
        Self {
            // `make_engine` runs inside the tokio runtime (during `App::new`),
            // so a current handle is available.
            handle: Handle::current(),
            sender: std::sync::Mutex::new(None),
        }
    }

    /// Return the actor's command sender, lazily spawning the actor on first use.
    fn sender(&self) -> Result<mpsc::Sender<Cmd>> {
        let mut guard = self.sender.lock().unwrap();
        if let Some(tx) = guard.as_ref() {
            return Ok(tx.clone());
        }
        let (tx, rx) = mpsc::channel::<Cmd>(8);
        self.handle.spawn(chrome_actor(rx));
        *guard = Some(tx.clone());
        Ok(tx)
    }
}

impl JsEngine for ChromeEngine {
    fn execute(&self, url: &Url, _html: &str) -> Result<JsResult> {
        let tx = self.sender()?;
        let (reply_tx, reply_rx) = oneshot::channel();

        // `execute` is sync; bridge into the async actor on the runtime. The
        // oneshot carries the actor's `Result`, so the channel-recv and the
        // actor result are flattened with `and_then`.
        let outcome: Result<(String, String)> = tokio::task::block_in_place(|| {
            self.handle.block_on(async move {
                tx.send(Cmd::Navigate {
                    url: url.to_string(),
                    reply: reply_tx,
                })
                .await
                .map_err(|_| anyhow!("Chrome actor terminated unexpectedly"))?;
                reply_rx
                    .await
                    .map_err(|_| anyhow!("Chrome actor dropped its reply"))
                    .and_then(|actor_result| actor_result)
            })
        });

        let (html, final_url) = outcome?;
        Ok(JsResult {
            html: Some(html),
            console: Vec::new(),
            final_url: Some(final_url),
        })
    }

    fn name(&self) -> &'static str {
        "chrome"
    }
}

/// Drain queued navigation commands, replying to each with `msg`, then return.
///
/// Used when the browser cannot be started, so callers waiting on `execute`
/// get a specific diagnostic instead of a bare "dropped reply".
async fn drain_with_error(mut rx: mpsc::Receiver<Cmd>, msg: String) {
    while let Some(cmd) = rx.recv().await {
        let Cmd::Navigate { reply, .. } = cmd;
        let _ = reply.send(Err(anyhow!("{}", msg)));
    }
}

/// Owns the browser and serves navigation commands until the channel closes.
async fn chrome_actor(mut rx: mpsc::Receiver<Cmd>) {
    // Launch headless Chrome. (Default `BrowserConfig` is headless and keeps the
    // OS sandbox enabled, which is appropriate for a locally-run browser.)
    let config = match BrowserConfig::builder().build() {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("Chrome config build failed: {e}");
            tracing::error!("{msg}");
            drain_with_error(rx, msg).await;
            return;
        }
    };
    let (browser, mut handler) = match Browser::launch(config).await {
        Ok(bh) => bh,
        Err(e) => {
            let msg = format!("Chrome launch failed (is Chrome/Chromium on PATH?): {e}");
            tracing::error!("{msg}");
            drain_with_error(rx, msg).await;
            return;
        }
    };

    // The CDP event loop must be polled continuously or the connection stalls.
    tokio::spawn(async move {
        while let Some(_event) = handler.next().await {}
        tracing::debug!("Chrome CDP handler stream ended");
    });

    // A single reusable page/tab for all navigations.
    let page = match browser.new_page("about:blank").await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Chrome new_page failed: {e}");
            return;
        }
    };

    while let Some(cmd) = rx.recv().await {
        let Cmd::Navigate { url, reply } = cmd;
        let _ = reply.send(navigate(&page, &url).await);
    }
    tracing::debug!("Chrome actor command channel closed; shutting down browser");
    // `browser` (and `page`) drop here, closing the browser process.
}

/// Navigate `page` to `url`, wait for JS to render, and return (html, final_url).
async fn navigate(page: &Page, url: &str) -> Result<(String, String)> {
    page.goto(url).await?;
    // `goto` awaits the page load event. chromiumoxide 0.7 has no networkIdle
    // wait helper, so allow a brief fixed window for client-side JS to render
    // before snapshotting the DOM. (Pragmatic v1 heuristic.)
    tokio::time::sleep(Duration::from_millis(500)).await;
    let html = page.content().await?;
    let final_url = page.url().await?.unwrap_or_else(|| url.to_string());
    Ok((html, final_url))
}

#[cfg(all(test, feature = "chrome-engine"))]
mod tests {
    use super::*;

    // Without Chrome/Chromium on PATH the actor must fail to launch and reply
    // with a clear error (drained through `drain_with_error`) rather than
    // hanging on the oneshot. If Chrome *is* present, accept a successful render
    // — the point is that `execute` returns and never hangs.
    #[tokio::test(flavor = "multi_thread")]
    async fn execute_returns_without_hanging() {
        // `block_in_place` requires a multi-threaded runtime (the flavor above).
        let engine = ChromeEngine::new();
        let url = Url::parse("https://example.com").unwrap();
        match engine.execute(&url, "") {
            Ok(r) => assert!(r.html.is_some(), "Chrome backend returned no HTML"),
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("Chrome"),
                    "expected a Chrome-related launch error, got: {msg}"
                );
            }
        }
    }
}
