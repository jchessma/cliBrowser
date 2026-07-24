use std::collections::HashMap;
use url::Url;

use crate::layout::{Block, Form, FormField, Link, TabItem};

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
    pub forms: Vec<Form>,
    pub form_fields: Vec<FormField>,
    pub tab_order: Vec<TabItem>,
    pub raw_html: String,
    pub status_code: u16,
    /// When true, the page is a `view-source:` view: `raw_html` is rendered
    /// directly as line-numbered source instead of `blocks`.
    pub is_source: bool,
}

pub struct BrowserState {
    pub current_page: Option<LoadedPage>,
    pub status: PageStatus,
    pub scroll: usize,
    /// Index into `current_page.tab_order` (covers both links and form fields).
    pub focused_tab: Option<usize>,
    /// When true, keyboard characters are routed to the focused text field.
    pub editing_field: bool,
    /// Current values of form fields, keyed by FormField::index.
    pub field_values: HashMap<usize, String>,
    pub js_engine_name: &'static str,
}

impl BrowserState {
    pub fn new(js_engine_name: &'static str) -> Self {
        Self {
            current_page: None,
            status: PageStatus::Ready,
            scroll: 0,
            focused_tab: None,
            editing_field: false,
            field_values: HashMap::new(),
            js_engine_name,
        }
    }

    pub fn reset_navigation(&mut self) {
        self.scroll = 0;
        self.focused_tab = None;
        self.editing_field = false;
        // Keep field_values in case the same form is reloaded (intentional)
        self.field_values.clear();
    }

    // ── Scrolling ──────────────────────────────────────────────────────────

    pub fn scroll_down(&mut self, lines: usize, total_lines: usize, viewport_height: usize) {
        let max_scroll = total_lines.saturating_sub(viewport_height);
        self.scroll = (self.scroll + lines).min(max_scroll);
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    // ── Tab order navigation ───────────────────────────────────────────────

    fn tab_len(&self) -> usize {
        self.current_page
            .as_ref()
            .map(|p| p.tab_order.len())
            .unwrap_or(0)
    }

    pub fn tab_next(&mut self) {
        let len = self.tab_len();
        if len == 0 {
            return;
        }
        self.focused_tab = Some(match self.focused_tab {
            None => 0,
            Some(i) => (i + 1) % len,
        });
        self.editing_field = false;
        self.auto_edit_text_field();
    }

    pub fn tab_prev(&mut self) {
        let len = self.tab_len();
        if len == 0 {
            return;
        }
        self.focused_tab = Some(match self.focused_tab {
            None => len.saturating_sub(1),
            Some(0) => len.saturating_sub(1),
            Some(i) => i - 1,
        });
        self.editing_field = false;
        self.auto_edit_text_field();
    }

    /// If the newly focused tab item is a text-entry field, automatically enter
    /// edit mode so the user can type immediately (Lynx-style behaviour).
    fn auto_edit_text_field(&mut self) {
        // Extract field index and default value without holding a borrow on self.
        let (fi, default) = match self.current_tab_item() {
            Some(TabItem::Field(fi)) => {
                let fi = *fi;
                let default = self
                    .current_page
                    .as_ref()
                    .and_then(|p| p.form_fields.get(fi))
                    .and_then(|f| match f.field_type {
                        crate::layout::FormFieldType::Text
                        | crate::layout::FormFieldType::Password
                        | crate::layout::FormFieldType::TextArea => Some(f.default_value.clone()),
                        _ => None,
                    });
                (fi, default)
            }
            _ => return,
        };
        if let Some(default) = default {
            self.field_values.entry(fi).or_insert(default);
            self.editing_field = true;
        }
    }

    pub fn current_tab_item(&self) -> Option<&TabItem> {
        let page = self.current_page.as_ref()?;
        let idx = self.focused_tab?;
        page.tab_order.get(idx)
    }

    /// If the currently focused tab item is a link, return its URL.
    pub fn focused_link_url(&self) -> Option<String> {
        let page = self.current_page.as_ref()?;
        match self.current_tab_item()? {
            TabItem::Link(li) => page.links.get(*li).map(|l| l.url.clone()),
            _ => None,
        }
    }

    /// If the currently focused tab item is a form field, return its index.
    pub fn focused_field_index(&self) -> Option<usize> {
        match self.current_tab_item()? {
            TabItem::Field(fi) => Some(*fi),
            _ => None,
        }
    }

    /// Return (link_index, field_index) for renderer highlighting.
    /// link_index: the raw link index if a link is focused.
    /// field_index: the FormField::index if a field is focused.
    pub fn focus_state(&self) -> (Option<usize>, Option<usize>) {
        match self.current_tab_item() {
            Some(TabItem::Link(li)) => (Some(*li), None),
            Some(TabItem::Field(fi)) => (None, Some(*fi)),
            None => (None, None),
        }
    }

    // ── Field value access ─────────────────────────────────────────────────

    /// Current (possibly edited) value for a field.
    pub fn field_value<'a>(&'a self, field: &'a FormField) -> &'a str {
        self.field_values
            .get(&field.index)
            .map(|s| s.as_str())
            .unwrap_or(&field.default_value)
    }

    pub fn field_value_mut(&mut self, field_index: usize) -> &mut String {
        self.field_values
            .entry(field_index)
            .or_insert_with(String::new)
    }

    // ── Form submission ────────────────────────────────────────────────────

    /// Collect all field name/value pairs for the form containing `field_index`.
    pub fn collect_form_data(&self, field_index: usize) -> Option<(usize, Vec<(String, String)>)> {
        let page = self.current_page.as_ref()?;
        let form_index = page.form_fields.get(field_index)?.form_index;

        let mut data: Vec<(String, String)> = Vec::new();
        for field in &page.form_fields {
            if field.form_index != form_index {
                continue;
            }
            match &field.field_type {
                crate::layout::FormFieldType::Hidden => {
                    data.push((field.name.clone(), field.default_value.clone()));
                }
                crate::layout::FormFieldType::Submit => {
                    // Only include the submit button value if it has a name
                    if !field.name.is_empty() && !field.default_value.is_empty() {
                        data.push((field.name.clone(), field.default_value.clone()));
                    }
                }
                crate::layout::FormFieldType::Checkbox { checked } => {
                    let is_checked = self
                        .field_values
                        .get(&field.index)
                        .map(|v| v == "on")
                        .unwrap_or(*checked);
                    if is_checked {
                        let value = if field.default_value.is_empty() {
                            "on".to_string()
                        } else {
                            field.default_value.clone()
                        };
                        data.push((field.name.clone(), value));
                    }
                }
                crate::layout::FormFieldType::Radio { checked } => {
                    let is_checked = self
                        .field_values
                        .get(&field.index)
                        .map(|v| v == "on")
                        .unwrap_or(*checked);
                    if is_checked {
                        data.push((field.name.clone(), field.default_value.clone()));
                    }
                }
                _ => {
                    let value = self
                        .field_values
                        .get(&field.index)
                        .cloned()
                        .unwrap_or_else(|| field.default_value.clone());
                    data.push((field.name.clone(), value));
                }
            }
        }
        Some((form_index, data))
    }
}
