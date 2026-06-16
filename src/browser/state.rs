use url::Url;

use crate::layout::{Block, Link};

#[derive(Debug, Clone, PartialEq)]
pub enum PageStatus {
    Ready,
    Loading,
    Error(String),
}

pub struct LoadedPage {
    pub url: Url,
    pub title: String,
    pub blocks: Vec<Block>,
    pub links: Vec<Link>,
    pub raw_html: String,
    pub status_code: u16,
}

pub struct BrowserState {
    pub current_page: Option<LoadedPage>,
    pub status: PageStatus,
    pub scroll: usize,
    pub focused_link: Option<usize>,
    pub js_engine_name: &'static str,
}

impl BrowserState {
    pub fn new(js_engine_name: &'static str) -> Self {
        Self {
            current_page: None,
            status: PageStatus::Ready,
            scroll: 0,
            focused_link: None,
            js_engine_name,
        }
    }

    pub fn reset_navigation(&mut self) {
        self.scroll = 0;
        self.focused_link = None;
    }

    pub fn scroll_down(&mut self, lines: usize, total_lines: usize, viewport_height: usize) {
        let max_scroll = total_lines.saturating_sub(viewport_height);
        self.scroll = (self.scroll + lines).min(max_scroll);
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    pub fn next_link(&mut self) {
        if let Some(page) = &self.current_page {
            let count = page.links.len();
            if count == 0 {
                return;
            }
            self.focused_link = Some(match self.focused_link {
                None => 0,
                Some(i) => (i + 1) % count,
            });
        }
    }

    pub fn prev_link(&mut self) {
        if let Some(page) = &self.current_page {
            let count = page.links.len();
            if count == 0 {
                return;
            }
            self.focused_link = Some(match self.focused_link {
                None => count.saturating_sub(1),
                Some(0) => count.saturating_sub(1),
                Some(i) => i - 1,
            });
        }
    }

    pub fn focused_url(&self) -> Option<String> {
        let page = self.current_page.as_ref()?;
        let idx = self.focused_link?;
        page.links.get(idx).map(|l| l.url.clone())
    }
}
