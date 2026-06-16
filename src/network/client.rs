use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, COOKIE};
use url::Url;

use super::CookieStore;

pub struct Response {
    pub url: Url,
    pub status: u16,
    pub content_type: String,
    pub body: String,
    pub set_cookies: Vec<String>,
}

pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self> {
        let inner = reqwest::Client::builder()
            .user_agent("clibrowser/0.1 (CLI; +https://github.com/user/clibrowser)")
            .redirect(reqwest::redirect::Policy::limited(10))
            .use_rustls_tls()
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self { inner })
    }

    pub async fn get(&self, url: &Url, cookies: &CookieStore) -> Result<Response> {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US,en;q=0.9"),
        );

        if let Some(cookie_header) = cookies.get_header(url) {
            if let Ok(v) = HeaderValue::from_str(&cookie_header) {
                headers.insert(COOKIE, v);
            }
        }

        let resp = self
            .inner
            .get(url.as_str())
            .headers(headers)
            .send()
            .await
            .context("HTTP request failed")?;

        let final_url = resp.url().clone();
        let status = resp.status().as_u16();

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let set_cookies: Vec<String> = resp
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(String::from))
            .collect();

        let body = resp.text().await.context("Failed to read response body")?;

        Ok(Response {
            url: final_url,
            status,
            content_type,
            body,
            set_cookies,
        })
    }

    pub async fn post(
        &self,
        url: &Url,
        body: &str,
        content_type: &str,
        cookies: &CookieStore,
    ) -> Result<Response> {
        let mut headers = HeaderMap::new();
        if let Some(cookie_header) = cookies.get_header(url) {
            if let Ok(v) = HeaderValue::from_str(&cookie_header) {
                headers.insert(COOKIE, v);
            }
        }

        let resp = self
            .inner
            .post(url.as_str())
            .headers(headers)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(body.to_string())
            .send()
            .await
            .context("HTTP POST failed")?;

        let final_url = resp.url().clone();
        let status = resp.status().as_u16();
        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let set_cookies: Vec<String> = resp
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(String::from))
            .collect();

        let body_text = resp.text().await.context("Failed to read POST response")?;

        Ok(Response {
            url: final_url,
            status,
            content_type: ct,
            body: body_text,
            set_cookies,
        })
    }
}
