use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CookieStore {
    // domain -> name -> value
    cookies: HashMap<String, HashMap<String, String>>,
}

impl CookieStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, domain: &str, name: &str, value: &str) {
        self.cookies
            .entry(domain.to_string())
            .or_default()
            .insert(name.to_string(), value.to_string());
    }

    pub fn get_header(&self, url: &Url) -> Option<String> {
        let domain = url.host_str()?;
        let jar = self.cookies.get(domain)?;
        if jar.is_empty() {
            return None;
        }
        let pairs: Vec<String> = jar.iter().map(|(k, v)| format!("{k}={v}")).collect();
        Some(pairs.join("; "))
    }

    pub fn parse_set_cookie(&mut self, url: &Url, header: &str) {
        let domain = match url.host_str() {
            Some(d) => d.to_string(),
            None => return,
        };
        // Simple parser: "name=value; attributes..."
        if let Some(pair) = header.split(';').next() {
            if let Some((name, value)) = pair.split_once('=') {
                self.set(&domain, name.trim(), value.trim());
            }
        }
    }
}
