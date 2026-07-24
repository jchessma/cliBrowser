use anyhow::Result;
use url::Url;

/// The result of running scripts on a page.
pub struct JsResult {
    /// Modified HTML after script execution (if available).
    pub html: Option<String>,
    /// Console output from scripts.
    pub console: Vec<String>,
    /// Final URL after the backend navigated (e.g. Chrome following redirects or
    /// JS-driven navigation). `None` for backends that don't navigate (QuickJS,
    /// None), which leaves the originally fetched URL in place.
    pub final_url: Option<String>,
}

/// Which JS backend to use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Backend {
    QuickJs,
    Chrome,
    None,
}

/// Trait for JS engine backends.
pub trait JsEngine: Send + Sync {
    /// Execute scripts in the given HTML document and return the result.
    fn execute(&self, url: &Url, html: &str) -> Result<JsResult>;

    fn name(&self) -> &'static str;
}

/// Create a JS engine for the given backend.
pub fn make_engine(backend: Backend) -> Box<dyn JsEngine> {
    match backend {
        #[cfg(feature = "quickjs-engine")]
        Backend::QuickJs => Box::new(super::quickjs::QuickJsEngine::new()),
        #[cfg(not(feature = "quickjs-engine"))]
        Backend::QuickJs => Box::new(super::none::NoEngine),
        #[cfg(feature = "chrome-engine")]
        Backend::Chrome => Box::new(super::chrome::ChromeEngine::new()),
        #[cfg(not(feature = "chrome-engine"))]
        Backend::Chrome => {
            tracing::warn!("Chrome engine requested but not compiled in; using no-JS fallback");
            Box::new(super::none::NoEngine)
        }
        Backend::None => Box::new(super::none::NoEngine),
    }
}
