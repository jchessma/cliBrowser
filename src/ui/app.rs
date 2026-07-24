use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span as RSpan},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use url::Url;

use crate::{
    browser::{Bookmarks, BrowserState, LoadedPage, PageStatus},
    js::{self, make_engine, Backend},
    layout as page_layout,
    network::{Client, CookieStore},
    renderer::{self, PageFocus},
};

use super::keybindings::Action;

struct App {
    state: BrowserState,
    history: crate::browser::History,
    bookmarks: Bookmarks,
    client: Client,
    cookies: CookieStore,
    js_engine: Box<dyn js::JsEngine>,
    url_bar_open: bool,
    url_bar_input: String,
    show_bookmarks: bool,
    show_help: bool,
    status_msg: Option<String>,
    total_rendered_lines: usize,
}

impl App {
    fn new(js_backend: Backend) -> Result<Self> {
        let engine = make_engine(js_backend);
        let engine_name = engine.name();
        Ok(Self {
            state: BrowserState::new(engine_name),
            history: crate::browser::History::new(),
            bookmarks: Bookmarks::load(),
            client: Client::new()?,
            cookies: CookieStore::new(),
            js_engine: engine,
            url_bar_open: false,
            url_bar_input: String::new(),
            show_bookmarks: false,
            show_help: false,
            status_msg: None,
            total_rendered_lines: 0,
        })
    }

    async fn navigate(&mut self, raw_url: &str) -> Result<()> {
        let url_str = normalize_url(raw_url);

        if url_str.starts_with("about:") {
            self.load_about(&url_str);
            self.history.navigate(url_str);
            return Ok(());
        }

        let url = Url::parse(&url_str)?;
        self.fetch_and_load(url, None).await
    }

    async fn submit_form(&mut self, field_index: usize) -> Result<()> {
        let (form_index, form_data) = match self.state.collect_form_data(field_index) {
            Some(d) => d,
            None => return Ok(()),
        };

        let page = match &self.state.current_page {
            Some(p) => p,
            None => return Ok(()),
        };

        let form = match page.forms.get(form_index) {
            Some(f) => f,
            None => return Ok(()),
        };

        let action = if form.action.is_empty() {
            page.url.to_string()
        } else {
            match page.url.join(&form.action) {
                Ok(u) => u.to_string(),
                Err(_) => form.action.clone(),
            }
        };

        let method = form.method.clone();
        let enctype = form.enctype.clone();

        match method {
            page_layout::FormMethod::Get => {
                let qs = encode_form_urlencoded(&form_data);
                let target = if qs.is_empty() {
                    action
                } else {
                    format!("{}?{}", action.split('?').next().unwrap_or(&action), qs)
                };
                self.navigate(&target).await?;
            }
            page_layout::FormMethod::Post => {
                let url = Url::parse(&action)?;
                let body = encode_form_urlencoded(&form_data);
                self.state.status = PageStatus::Loading;
                self.status_msg = Some(format!("Submitting to {}…", action));

                let resp = self.client.post(&url, &body, &enctype, &self.cookies).await?;
                for cookie in &resp.set_cookies {
                    self.cookies.parse_set_cookie(&resp.url, cookie);
                }
                self.load_response(resp)?;
                self.history.navigate(url.to_string());
            }
        }

        Ok(())
    }

    async fn fetch_and_load(&mut self, url: Url, post_body: Option<(&str, &str)>) -> Result<()> {
        self.state.status = PageStatus::Loading;
        self.status_msg = Some(format!("Loading {}…", url));

        let resp = match post_body {
            Some((body, ct)) => self.client.post(&url, body, ct, &self.cookies).await,
            None => self.client.get(&url, &self.cookies).await,
        };

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                self.state.status = PageStatus::Error(e.to_string());
                self.status_msg = Some(format!("Error: {}", e));
                return Err(e);
            }
        };

        for cookie in &resp.set_cookies {
            self.cookies.parse_set_cookie(&resp.url, cookie);
        }

        let final_url = resp.url.clone();
        self.history.navigate(final_url.to_string());
        self.load_response(resp)?;
        Ok(())
    }

    fn load_response(&mut self, resp: crate::network::Response) -> Result<()> {
        let html = match self.js_engine.execute(&resp.url, &resp.body) {
            Ok(r) => r.html.unwrap_or(resp.body.clone()),
            Err(e) => {
                tracing::warn!("JS execution error: {}", e);
                resp.body.clone()
            }
        };

        let doc = crate::dom::parse_html(&html);
        let js_enabled = self.js_engine.name() != "none";
        let layout = page_layout::layout_with_opts(&doc.root, js_enabled);

        let title = if doc.title.is_empty() {
            resp.url.host_str().unwrap_or("untitled").to_string()
        } else {
            doc.title.clone()
        };

        self.state.current_page = Some(LoadedPage {
            url: resp.url,
            title,
            blocks: layout.blocks,
            links: layout.links,
            forms: layout.forms,
            form_fields: layout.form_fields,
            tab_order: layout.tab_order,
            raw_html: resp.body,
            status_code: resp.status,
        });
        self.state.reset_navigation();
        self.state.status = PageStatus::Ready;
        self.status_msg = None;
        Ok(())
    }

    fn load_about(&mut self, url: &str) {
        let blocks = match url {
            "about:home" | "about:blank" => home_page_blocks(),
            _ => {
                use crate::layout::Block as LBlock;
                vec![LBlock::Paragraph(vec![vec![plain_span(&format!(
                    "Unknown about: page: {}",
                    url
                ))]])]
            }
        };

        self.state.current_page = Some(LoadedPage {
            url: Url::parse(url).unwrap_or_else(|_| Url::parse("about:blank").unwrap()),
            title: "Home".to_string(),
            blocks,
            links: Vec::new(),
            forms: Vec::new(),
            form_fields: Vec::new(),
            tab_order: Vec::new(),
            raw_html: String::new(),
            status_code: 200,
        });
        self.state.reset_navigation();
        self.state.status = PageStatus::Ready;
        self.status_msg = None;
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        self.draw_url_bar(frame, chunks[0]);
        self.draw_content(frame, chunks[1]);
        self.draw_status_bar(frame, chunks[2]);

        if self.url_bar_open {
            self.draw_url_input(frame, area);
        }
        if self.show_help {
            self.draw_help(frame, area);
        }
        if self.show_bookmarks {
            self.draw_bookmarks(frame, area);
        }
    }

    fn draw_url_bar(&self, frame: &mut Frame, area: Rect) {
        let back_style = if self.history.can_go_back() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let fwd_style = if self.history.can_go_forward() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let url_text = self
            .state
            .current_page
            .as_ref()
            .map(|p| p.url.to_string())
            .unwrap_or_default();

        let is_bookmarked = self.bookmarks.contains(&url_text);
        let (loading_char, loading_color) = match self.state.status {
            PageStatus::Loading => ("⟳", Color::Yellow),
            PageStatus::Error(_) => ("✗", Color::Red),
            PageStatus::Ready => (" ", Color::DarkGray),
        };

        let bar = Line::from(vec![
            RSpan::styled(" ◀ ", back_style),
            RSpan::styled(" ▶ ", fwd_style),
            RSpan::styled(
                format!(" {} ", loading_char),
                Style::default().fg(loading_color),
            ),
            RSpan::styled(
                format!(" {} ", url_text),
                Style::default().fg(Color::White).bg(Color::DarkGray),
            ),
            RSpan::styled(
                if is_bookmarked { " ★ " } else { " ☆ " },
                Style::default().fg(if is_bookmarked {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }),
            ),
        ]);

        frame.render_widget(Paragraph::new(bar), area);
    }

    fn draw_content(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(page) = &self.state.current_page {
            let (focused_link, focused_field) = self.state.focus_state();
            let focus = PageFocus {
                link: focused_link,
                field: focused_field,
                editing: self.state.editing_field,
                field_values: &self.state.field_values,
            };

            let text = renderer::render(&page.blocks, &page.links, area.width, &focus);
            self.total_rendered_lines = text.lines.len();

            frame.render_widget(
                Paragraph::new(text)
                    .scroll((self.state.scroll as u16, 0))
                    .wrap(Wrap { trim: false }),
                area,
            );
        } else {
            let msg = match &self.state.status {
                PageStatus::Loading => "Loading…",
                PageStatus::Error(e) => e.as_str(),
                PageStatus::Ready => "Press 'o' to open a URL, '?' for help",
            };
            frame.render_widget(
                Paragraph::new(msg).style(Style::default().fg(Color::DarkGray)),
                area,
            );
        }
    }

    fn draw_status_bar(&self, frame: &mut Frame, area: Rect) {
        let focus_hint = match self.state.current_tab_item() {
            Some(crate::layout::TabItem::Link(li)) => {
                if let Some(page) = &self.state.current_page {
                    page.links.get(*li).map(|l| format!(" → {}", l.url)).unwrap_or_default()
                } else {
                    String::new()
                }
            }
            Some(crate::layout::TabItem::Field(fi)) => {
                if self.state.editing_field {
                    format!(" [typing — Tab to move, Enter to submit, Esc to cancel]")
                } else {
                    format!(" [form field — Enter to submit, Tab to move]")
                }
            }
            None => String::new(),
        };

        let title = self
            .state
            .current_page
            .as_ref()
            .map(|p| p.title.as_str())
            .unwrap_or("");

        let status_text = self.status_msg.as_deref().unwrap_or(title);
        let js_indicator = format!("[JS:{}]", self.state.js_engine_name);

        let bar_text = format!(" {}{} {} ", status_text, focus_hint, js_indicator);

        frame.render_widget(
            Paragraph::new(bar_text)
                .style(Style::default().fg(Color::Black).bg(Color::Blue)),
            area,
        );
    }

    fn draw_url_input(&self, frame: &mut Frame, area: Rect) {
        let width = area.width.min(80);
        let x = area.width.saturating_sub(width) / 2;
        let popup = Rect { x, y: 2, width, height: 3 };
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(" Open: {}_", self.url_bar_input))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(" Navigate (Enter to go, Esc to cancel) "),
                )
                .style(Style::default().fg(Color::White).bg(Color::DarkGray)),
            popup,
        );
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let width = 64u16.min(area.width);
        let height = 32u16.min(area.height);
        let x = area.width.saturating_sub(width) / 2;
        let y = area.height.saturating_sub(height) / 2;
        let popup = Rect { x, y, width, height };

        let help = [
            "  Navigation",
            "  ─────────────────────────────────────────",
            "  j / ↓         Scroll down one line",
            "  k / ↑         Scroll up one line",
            "  d / u         Scroll half page down/up",
            "  Space         Page down",
            "  PgUp          Page up",
            "  g             Go to top",
            "  G             Go to bottom",
            "",
            "  Links & Forms",
            "  ─────────────────────────────────────────",
            "  Tab           Next link / form field",
            "  Shift+Tab     Previous link / form field",
            "  Enter         Follow link / submit form",
            "  Space         Toggle checkbox when focused",
            "  Esc           Stop editing / close overlay",
            "  (text fields enter edit mode automatically)",
            "",
            "  Browser",
            "  ─────────────────────────────────────────",
            "  o             Open URL or search",
            "  r / F5        Reload page",
            "  Backspace     Back",
            "  H / Alt+←     Back",
            "  L / Alt+→     Forward",
            "  b             Toggle bookmark",
            "  B             Show bookmarks",
            "  y             Copy link URL",
            "  ?             Toggle this help",
            "  q / Ctrl+C    Quit",
        ]
        .join("\n");

        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(help)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(" Help — press ? to close "),
                )
                .style(Style::default().fg(Color::White).bg(Color::Black)),
            popup,
        );
    }

    fn draw_bookmarks(&self, frame: &mut Frame, area: Rect) {
        let width = 72u16.min(area.width);
        let height = 20u16.min(area.height);
        let x = area.width.saturating_sub(width) / 2;
        let y = area.height.saturating_sub(height) / 2;
        let popup = Rect { x, y, width, height };
        frame.render_widget(Clear, popup);

        if self.bookmarks.items.is_empty() {
            frame.render_widget(
                Paragraph::new("\n  No bookmarks yet.\n\n  Press 'b' to bookmark the current page.")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .title(" Bookmarks — B to close "),
                    )
                    .style(Style::default().fg(Color::DarkGray).bg(Color::Black)),
                popup,
            );
        } else {
            let items: Vec<ListItem> = self
                .bookmarks
                .items
                .iter()
                .map(|bm| {
                    let title_len = bm.title.len().min(28);
                    ListItem::new(Line::from(vec![
                        RSpan::styled(
                            format!("  {:<28} ", &bm.title[..title_len]),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                        RSpan::raw(bm.url.clone()),
                    ]))
                })
                .collect();

            frame.render_widget(
                List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .title(" Bookmarks — B to close "),
                    )
                    .style(Style::default().bg(Color::Black)),
                popup,
            );
        }
    }

    async fn handle_action(&mut self, action: Action, viewport_height: usize) -> Result<bool> {
        match action {
            Action::Quit => return Ok(true),

            Action::ScrollDown(n) => {
                self.state.scroll_down(n, self.total_rendered_lines, viewport_height);
            }
            Action::ScrollUp(n) => {
                self.state.scroll_up(n);
            }
            Action::PageDown => {
                self.state.scroll_down(viewport_height, self.total_rendered_lines, viewport_height);
            }
            Action::PageUp => {
                self.state.scroll_up(viewport_height);
            }
            Action::GoToTop => {
                self.state.scroll = 0;
            }
            Action::GoToBottom => {
                self.state.scroll = self.total_rendered_lines.saturating_sub(viewport_height);
            }

            Action::NextLink => {
                self.state.tab_next();
            }
            Action::PrevLink => {
                self.state.tab_prev();
            }

            Action::Follow => {
                self.handle_follow().await?;
            }

            // Space toggles checkbox/radio when focused; otherwise acts as page down
            Action::ToggleCheck => {
                let toggled = if let Some(fi) = self.state.focused_field_index() {
                    let field_type = self
                        .state
                        .current_page
                        .as_ref()
                        .and_then(|p| p.form_fields.get(fi))
                        .map(|f| f.field_type.clone());
                    match field_type {
                        Some(crate::layout::FormFieldType::Checkbox { checked }) => {
                            let cur = self
                                .state
                                .field_values
                                .get(&fi)
                                .map(|v| v == "on")
                                .unwrap_or(checked);
                            self.state
                                .field_values
                                .insert(fi, if cur { "off".into() } else { "on".into() });
                            true
                        }
                        Some(crate::layout::FormFieldType::Radio { .. }) => {
                            self.state.field_values.insert(fi, "on".into());
                            true
                        }
                        _ => false,
                    }
                } else {
                    false
                };
                if !toggled {
                    self.state.scroll_down(viewport_height, self.total_rendered_lines, viewport_height);
                }
            }

            Action::Back => {
                if let Some(url) = self.history.back() {
                    if let Err(e) = self.navigate_to_history(&url).await {
                        self.status_msg = Some(format!("Error: {}", e));
                    }
                }
            }
            Action::Forward => {
                if let Some(url) = self.history.forward() {
                    if let Err(e) = self.navigate_to_history(&url).await {
                        self.status_msg = Some(format!("Error: {}", e));
                    }
                }
            }
            Action::Reload => {
                if let Some(url) = self.history.current().map(String::from) {
                    if let Err(e) = self.navigate(&url).await {
                        self.status_msg = Some(format!("Error: {}", e));
                    }
                }
            }

            Action::Bookmark => {
                if let Some(page) = &self.state.current_page {
                    let url = page.url.to_string();
                    let title = page.title.clone();
                    if self.bookmarks.contains(&url) {
                        self.bookmarks.remove(&url);
                        self.status_msg = Some("Bookmark removed".to_string());
                    } else {
                        self.bookmarks.add(title, url);
                        self.status_msg = Some("Bookmarked!".to_string());
                    }
                    let _ = self.bookmarks.save();
                }
            }
            Action::ShowBookmarks => {
                self.show_bookmarks = !self.show_bookmarks;
            }
            Action::ShowHelp => {
                self.show_help = !self.show_help;
            }
            Action::OpenUrl => {
                self.url_bar_open = true;
                self.url_bar_input.clear();
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_follow(&mut self) -> Result<()> {
        match self.state.current_tab_item().cloned() {
            Some(crate::layout::TabItem::Link(li)) => {
                let href = self
                    .state
                    .current_page
                    .as_ref()
                    .and_then(|p| p.links.get(li))
                    .map(|l| l.url.clone());
                if let Some(href) = href {
                    let resolved = self.resolve_url(&href);
                    if let Err(e) = self.navigate(&resolved).await {
                        self.status_msg = Some(format!("Error: {}", e));
                        self.state.status = PageStatus::Ready;
                    }
                }
            }
            Some(crate::layout::TabItem::Field(fi)) => {
                let field_type = self
                    .state
                    .current_page
                    .as_ref()
                    .and_then(|p| p.form_fields.get(fi))
                    .map(|f| f.field_type.clone());

                match field_type {
                    Some(crate::layout::FormFieldType::Submit) => {
                        if let Err(e) = self.submit_form(fi).await {
                            self.status_msg = Some(format!("Submit error: {}", e));
                            self.state.status = PageStatus::Ready;
                        }
                    }
                    Some(
                        crate::layout::FormFieldType::Text
                        | crate::layout::FormFieldType::Password
                        | crate::layout::FormFieldType::TextArea,
                    ) => {
                        // Text fields auto-enter edit mode on Tab. Enter here submits the form.
                        self.state.editing_field = false;
                        if let Err(e) = self.submit_form(fi).await {
                            self.status_msg = Some(format!("Submit error: {}", e));
                            self.state.status = PageStatus::Ready;
                        }
                    }
                    Some(crate::layout::FormFieldType::Checkbox { checked }) => {
                        let cur = self
                            .state
                            .field_values
                            .get(&fi)
                            .map(|v| v == "on")
                            .unwrap_or(checked);
                        self.state
                            .field_values
                            .insert(fi, if cur { "off".into() } else { "on".into() });
                    }
                    Some(crate::layout::FormFieldType::Radio { .. }) => {
                        self.state.field_values.insert(fi, "on".into());
                    }
                    Some(crate::layout::FormFieldType::Select) => {
                        // Cycle through options
                        let (default_val, options) = self
                            .state
                            .current_page
                            .as_ref()
                            .and_then(|p| p.form_fields.get(fi))
                            .map(|f| (f.default_value.clone(), f.options.clone()))
                            .unwrap_or_default();
                        let current = self
                            .state
                            .field_values
                            .get(&fi)
                            .cloned()
                            .unwrap_or(default_val);
                        let next_idx = options
                            .iter()
                            .position(|(v, _)| *v == current)
                            .map(|i| (i + 1) % options.len())
                            .unwrap_or(0);
                        if let Some((v, _)) = options.get(next_idx) {
                            self.state.field_values.insert(fi, v.clone());
                        }
                    }
                    _ => {}
                }
            }
            None => {}
        }
        Ok(())
    }

    /// Handle raw key events when a text field is in edit mode.
    /// Called instead of map_key → handle_action so that ALL printable chars
    /// (including reserved browser keys like q, j, k, g …) reach the field.
    async fn handle_field_key(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        use crossterm::event::KeyModifiers;

        // Ctrl+C always quits
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            return Ok(true);
        }

        match key.code {
            KeyCode::Enter => {
                self.state.editing_field = false;
                if let Some(fi) = self.state.focused_field_index() {
                    if let Err(e) = self.submit_form(fi).await {
                        self.status_msg = Some(format!("Submit error: {}", e));
                        self.state.status = PageStatus::Ready;
                    }
                }
            }
            KeyCode::Tab => {
                self.state.editing_field = false;
                self.state.tab_next();
            }
            KeyCode::BackTab => {
                self.state.editing_field = false;
                self.state.tab_prev();
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if let Some(fi) = self.state.focused_field_index() {
                    self.state.field_value_mut(fi).pop();
                }
            }
            KeyCode::Char(c) => {
                if let Some(fi) = self.state.focused_field_index() {
                    self.state.field_value_mut(fi).push(c);
                }
            }
            // Arrow keys, Home, End — no cursor movement yet; ignore silently
            _ => {}
        }
        Ok(false)
    }

    fn resolve_url(&self, href: &str) -> String {
        if href.starts_with("http://") || href.starts_with("https://") {
            return href.to_string();
        }
        if let Some(page) = &self.state.current_page {
            if let Ok(resolved) = page.url.join(href) {
                return resolved.to_string();
            }
        }
        href.to_string()
    }

    async fn navigate_to_history(&mut self, url: &str) -> Result<()> {
        if url.starts_with("about:") {
            self.load_about(url);
            return Ok(());
        }
        let parsed = Url::parse(url)?;
        self.fetch_and_load(parsed, None).await
    }

    async fn handle_url_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.url_bar_open = false;
                self.url_bar_input.clear();
            }
            KeyCode::Enter => {
                let url = std::mem::take(&mut self.url_bar_input);
                self.url_bar_open = false;
                if !url.is_empty() {
                    if let Err(e) = self.navigate(&url).await {
                        self.status_msg = Some(format!("Error: {}", e));
                        self.state.status = PageStatus::Ready;
                    }
                }
            }
            KeyCode::Backspace => {
                self.url_bar_input.pop();
            }
            KeyCode::Char(c) => {
                self.url_bar_input.push(c);
            }
            _ => {}
        }
        Ok(())
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn normalize_url(raw: &str) -> String {
    if raw.starts_with("http://")
        || raw.starts_with("https://")
        || raw.starts_with("about:")
    {
        return raw.to_string();
    }
    // Heuristic: if it looks like a domain (contains a dot or slash), treat as URL
    if raw.contains('.') || raw.starts_with('/') {
        return format!("https://{}", raw);
    }
    // Otherwise search DuckDuckGo Lite
    format!(
        "https://lite.duckduckgo.com/lite/?q={}",
        percent_encoding::utf8_percent_encode(raw, percent_encoding::NON_ALPHANUMERIC)
    )
}

fn encode_form_urlencoded(fields: &[(String, String)]) -> String {
    use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
    // Form URL encoding encodes spaces as '+' and most special chars
    const FORM_ENCODE: &AsciiSet = &CONTROLS
        .add(b' ')
        .add(b'!')
        .add(b'"')
        .add(b'#')
        .add(b'$')
        .add(b'%')
        .add(b'&')
        .add(b'\'')
        .add(b'(')
        .add(b')')
        .add(b'*')
        .add(b'+')
        .add(b',')
        .add(b'/')
        .add(b':')
        .add(b';')
        .add(b'<')
        .add(b'=')
        .add(b'>')
        .add(b'?')
        .add(b'@')
        .add(b'[')
        .add(b'\\')
        .add(b']')
        .add(b'^')
        .add(b'`')
        .add(b'{')
        .add(b'|')
        .add(b'}')
        .add(b'~');

    fields
        .iter()
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, v)| {
            let k_enc = utf8_percent_encode(k, FORM_ENCODE).to_string().replace("%20", "+");
            let v_enc = utf8_percent_encode(v, FORM_ENCODE).to_string().replace("%20", "+");
            format!("{}={}", k_enc, v_enc)
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn plain_span(text: &str) -> crate::layout::Span {
    crate::layout::Span {
        text: text.to_string(),
        bold: false,
        italic: false,
        underline: false,
        strikethrough: false,
        link_index: None,
        fg: None,
        bg: None,
    }
}

fn home_page_blocks() -> Vec<crate::layout::Block> {
    use crate::layout::Block as LBlock;
    vec![
        LBlock::Spacer,
        LBlock::Heading {
            level: 1,
            line: vec![plain_span("clibrowser")],
        },
        LBlock::Paragraph(vec![vec![plain_span(
            "A modern CLI web browser — fast, keyboard-driven, JavaScript-aware.",
        )]]),
        LBlock::Spacer,
        LBlock::HRule,
        LBlock::Spacer,
        LBlock::Heading {
            level: 2,
            line: vec![plain_span("Quick Start")],
        },
        LBlock::Paragraph(vec![
            vec![plain_span("  o            Open a URL or search query")],
            vec![plain_span("  Tab          Cycle through links and form fields")],
            vec![plain_span("  Enter        Follow link  /  edit or submit field")],
            vec![plain_span("  Space        Toggle checkbox when focused")],
            vec![plain_span("  H / L        Back / Forward")],
            vec![plain_span("  b            Bookmark this page")],
            vec![plain_span("  ?            Show full help")],
            vec![plain_span("  q            Quit")],
        ]),
        LBlock::Spacer,
        LBlock::HRule,
        LBlock::Spacer,
        LBlock::Paragraph(vec![vec![plain_span(
            "Tip: type a search term (not a URL) and it goes to DuckDuckGo Lite.",
        )]]),
    ]
}

pub async fn run(start_url: String, js_backend: Backend) -> anyhow::Result<()> {
    let mut app = App::new(js_backend)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    if let Err(e) = app.navigate(&start_url).await {
        app.status_msg = Some(format!("Error: {}", e));
        app.state.status = PageStatus::Ready;
        app.load_about("about:home");
    }

    let result = event_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        let viewport_height = terminal.size().map(|s| s.height as usize).unwrap_or(24) - 2;

        terminal.draw(|f| app.draw(f))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.url_bar_open {
                        app.handle_url_input(key).await?;
                        continue;
                    }

                    // Esc cancels field editing or closes overlays
                    if key.code == KeyCode::Esc {
                        if app.state.editing_field {
                            app.state.editing_field = false;
                        } else {
                            app.show_help = false;
                            app.show_bookmarks = false;
                        }
                        continue;
                    }

                    // When editing a text field, bypass the action mapper so all
                    // printable characters (including reserved browser keys) reach the field.
                    if app.state.editing_field {
                        if app.handle_field_key(key).await? {
                            return Ok(());
                        }
                        continue;
                    }

                    let action = super::keybindings::map_key(key);
                    if app.handle_action(action, viewport_height).await? {
                        return Ok(());
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
}
