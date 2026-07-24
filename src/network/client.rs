use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, COOKIE};
use url::Url;

use super::CookieStore;

pub struct Response {
    pub url: Url,
    pub status: u16,
    pub content_type: String,
    /// Text body for HTML/text responses (charset-decoded by reqwest).
    pub body: String,
    /// Raw bytes for non-HTML (binary) responses, when `body` is empty.
    pub body_bytes: Option<Vec<u8>>,
    pub set_cookies: Vec<String>,
}

/// True for content types we attempt to render as a page.
///
/// Splits off `;` parameters, lowercases, and accepts only empty (treated as
/// `text/html` by the client), `text/html`, and `application/xhtml+xml`.
/// Everything else (text/plain, xml, json, images, pdf, octet-stream, …) is
/// considered non-HTML and triggers a download prompt instead.
pub fn is_html_content_type(ct: &str) -> bool {
    let essence = ct.split(';').next().unwrap_or(ct).trim().to_ascii_lowercase();
    essence.is_empty() || essence == "text/html" || essence == "application/xhtml+xml"
}

/// Read a reqwest response body as text (for HTML) or bytes (for binaries),
/// branching on the content type. HTML keeps reqwest's charset decoding;
/// binary bodies round-trip intact instead of erroring on invalid UTF-8.
async fn read_body(
    resp: reqwest::Response,
    content_type: &str,
) -> Result<(String, Option<Vec<u8>>)> {
    if is_html_content_type(content_type) {
        let body = resp.text().await.context("Failed to read response body")?;
        Ok((body, None))
    } else {
        let bytes = resp
            .bytes()
            .await
            .context("Failed to read response body")?
            .to_vec();
        Ok((String::new(), Some(bytes)))
    }
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

        let (body, body_bytes) = read_body(resp, &content_type).await?;

        Ok(Response {
            url: final_url,
            status,
            content_type,
            body,
            body_bytes,
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

        let (body_text, body_bytes) = read_body(resp, &ct).await?;

        Ok(Response {
            url: final_url,
            status,
            content_type: ct,
            body: body_text,
            body_bytes,
            set_cookies,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::is_html_content_type;

    #[test]
    fn html_content_types() {
        assert!(is_html_content_type("text/html"));
        assert!(is_html_content_type("text/html; charset=utf-8"));
        assert!(is_html_content_type("application/xhtml+xml"));
        assert!(is_html_content_type("")); // absent header defaults to html
    }

    #[test]
    fn non_html_content_types() {
        assert!(!is_html_content_type("text/plain"));
        assert!(!is_html_content_type("application/json"));
        assert!(!is_html_content_type("application/pdf"));
        assert!(!is_html_content_type("image/png"));
        assert!(!is_html_content_type("application/octet-stream"));
        assert!(!is_html_content_type("application/xml"));
    }
}
