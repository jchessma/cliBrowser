use anyhow::Result;
use url::Url;

use super::engine::{JsEngine, JsResult};

/// Chrome headless backend via Chrome DevTools Protocol.
/// Requires `google-chrome`, `chromium`, or `chromium-browser` on PATH.
pub struct ChromeEngine;

impl ChromeEngine {
    pub fn new() -> Self {
        Self
    }
}

impl JsEngine for ChromeEngine {
    fn execute(&self, url: &Url, _html: &str) -> Result<JsResult> {
        // TODO: spawn chromiumoxide browser, navigate to URL, wait for
        // networkidle, extract document.documentElement.outerHTML.
        // This requires an async context; the synchronous trait wrapper
        // will use block_in_place or a dedicated tokio runtime.
        tracing::warn!("Chrome JS backend: not yet implemented; returning raw HTML");
        Ok(JsResult {
            html: None,
            console: vec![format!(
                "Chrome backend stub: would have loaded {}",
                url.as_str()
            )],
        })
    }

    fn name(&self) -> &'static str {
        "chrome"
    }
}
