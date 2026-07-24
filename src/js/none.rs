use anyhow::Result;
use url::Url;

use super::engine::{JsEngine, JsResult};

pub struct NoEngine;

impl JsEngine for NoEngine {
    fn execute(&self, _url: &Url, _html: &str) -> Result<JsResult> {
        Ok(JsResult {
            html: None,
            console: Vec::new(),
            final_url: None,
        })
    }

    fn name(&self) -> &'static str {
        "none"
    }
}
