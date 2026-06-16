use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Bookmarks {
    pub items: Vec<Bookmark>,
}

impl Bookmarks {
    pub fn load() -> Self {
        Self::load_from_path(&Self::path()).unwrap_or_default()
    }

    fn load_from_path(path: &PathBuf) -> Result<Self> {
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn add(&mut self, title: String, url: String) {
        if !self.items.iter().any(|b| b.url == url) {
            self.items.push(Bookmark { title, url });
        }
    }

    pub fn remove(&mut self, url: &str) {
        self.items.retain(|b| b.url != url);
    }

    pub fn contains(&self, url: &str) -> bool {
        self.items.iter().any(|b| b.url == url)
    }

    fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clibrowser")
            .join("bookmarks.json")
    }
}
