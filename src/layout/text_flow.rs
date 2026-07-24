use crate::css::{self, Display, FontStyle, FontWeight, TextDecoration, WhiteSpace};
use crate::dom::{Node, NodeData};

/// A single styled run of text within a line.
#[derive(Debug, Clone)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub link_index: Option<usize>,
    pub fg: Option<css::TermColor>,
    pub bg: Option<css::TermColor>,
}

impl Span {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            link_index: None,
            fg: None,
            bg: None,
        }
    }
}

/// A line of inline content (one or more spans).
pub type Inline = Vec<Span>;

/// A link extracted from the page.
#[derive(Debug, Clone)]
pub struct Link {
    pub text: String,
    pub url: String,
}

/// A `<form>` element's metadata.
#[derive(Debug, Clone)]
pub struct Form {
    pub action: String,
    pub method: FormMethod,
    pub enctype: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormMethod {
    Get,
    Post,
}

/// An input field in a form.
#[derive(Debug, Clone)]
pub struct FormField {
    /// Unique index across all fields on the page (used for stable identity).
    pub index: usize,
    /// Which form this field belongs to.
    pub form_index: usize,
    pub field_type: FormFieldType,
    pub name: String,
    /// Default / initial value from HTML.
    pub default_value: String,
    pub options: Vec<(String, String)>, // (value, label) for <select>
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormFieldType {
    Text,
    Password,
    TextArea,
    Submit,
    Checkbox { checked: bool },
    Radio { checked: bool },
    Select,
    Hidden,
}

/// A logical block of content (paragraph, heading, list item, etc.)
#[derive(Debug, Clone)]
pub enum Block {
    /// Plain wrapped paragraph / div
    Paragraph(Vec<Inline>),
    /// Heading (level 1–6) with styled line
    Heading { level: u8, line: Inline },
    /// Horizontal rule
    HRule,
    /// Preformatted text block (lines are preserved as-is)
    Pre(Vec<String>),
    /// Blockquote wrapping inner blocks
    Blockquote(Vec<Block>),
    /// List item (unordered or ordered)
    ListItem {
        ordered: bool,
        index: usize,
        content: Vec<Inline>,
    },
    /// Table row of cells
    TableRow(Vec<Vec<Inline>>),
    /// A form input field
    FormField(FormField),
    /// Blank line spacer
    Spacer,
}

pub struct LayoutResult {
    pub blocks: Vec<Block>,
    pub links: Vec<Link>,
    pub forms: Vec<Form>,
    /// All form fields in document order, indexed by FormField::index.
    pub form_fields: Vec<FormField>,
    /// Tab order: interleaved links and fields in document order.
    pub tab_order: Vec<TabItem>,
}

/// A focusable element reachable via Tab.
#[derive(Debug, Clone, PartialEq)]
pub enum TabItem {
    Link(usize),
    Field(usize),
}

/// State threaded through the walk.
struct Ctx {
    links: Vec<Link>,
    forms: Vec<Form>,
    form_fields: Vec<FormField>,
    list_stack: Vec<(bool, usize)>, // (ordered, counter)
    js_enabled: bool,
    /// Stack of form indices as we descend into nested form contexts.
    form_stack: Vec<usize>,
}

pub fn layout(doc: &Node) -> LayoutResult {
    layout_with_opts(doc, true)
}

pub fn layout_with_opts(doc: &Node, js_enabled: bool) -> LayoutResult {
    let mut ctx = Ctx {
        links: Vec::new(),
        forms: Vec::new(),
        form_fields: Vec::new(),
        list_stack: Vec::new(),
        js_enabled,
        form_stack: Vec::new(),
    };

    let mut blocks = Vec::new();
    walk(doc, &mut ctx, &mut blocks, &StyleState::default());

    let blocks = collapse_spacers(blocks);
    let tab_order = extract_tab_order(&blocks);

    LayoutResult {
        blocks,
        links: ctx.links,
        forms: ctx.forms,
        form_fields: ctx.form_fields,
        tab_order,
    }
}

/// Walk blocks collecting focusable elements in document order.
fn extract_tab_order(blocks: &[Block]) -> Vec<TabItem> {
    let mut order = Vec::new();
    let mut seen_links = std::collections::HashSet::new();
    collect_tab_items(blocks, &mut order, &mut seen_links);
    order
}

fn collect_tab_items(
    blocks: &[Block],
    order: &mut Vec<TabItem>,
    seen: &mut std::collections::HashSet<usize>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(lines) | Block::ListItem { content: lines, .. } => {
                for line in lines {
                    for span in line {
                        if let Some(idx) = span.link_index {
                            if seen.insert(idx) {
                                order.push(TabItem::Link(idx));
                            }
                        }
                    }
                }
            }
            Block::Heading { line, .. } => {
                for span in line {
                    if let Some(idx) = span.link_index {
                        if seen.insert(idx) {
                            order.push(TabItem::Link(idx));
                        }
                    }
                }
            }
            Block::TableRow(cells) => {
                for cell in cells {
                    for line in cell {
                        for span in line {
                            if let Some(idx) = span.link_index {
                                if seen.insert(idx) {
                                    order.push(TabItem::Link(idx));
                                }
                            }
                        }
                    }
                }
            }
            Block::Blockquote(inner) => {
                collect_tab_items(inner, order, seen);
            }
            Block::FormField(f) if f.field_type != FormFieldType::Hidden => {
                order.push(TabItem::Field(f.index));
            }
            _ => {}
        }
    }
}

/// Collapse consecutive Spacers and strip leading/trailing ones.
fn collapse_spacers(blocks: Vec<Block>) -> Vec<Block> {
    let mut result: Vec<Block> = Vec::with_capacity(blocks.len());
    let mut last_spacer = false;
    for block in blocks {
        match block {
            Block::Spacer => {
                if !last_spacer && !result.is_empty() {
                    result.push(Block::Spacer);
                    last_spacer = true;
                }
            }
            other => {
                last_spacer = false;
                result.push(other);
            }
        }
    }
    // Remove trailing spacer
    if matches!(result.last(), Some(Block::Spacer)) {
        result.pop();
    }
    result
}

/// Inherited style state during tree walk.
#[derive(Debug, Clone, Default)]
struct StyleState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    link_index: Option<usize>,
    fg: Option<css::TermColor>,
    in_pre: bool,
}

fn walk(node: &Node, ctx: &mut Ctx, blocks: &mut Vec<Block>, style: &StyleState) {
    match &node.data {
        NodeData::Text(text) => {
            let t = if style.in_pre {
                text.clone()
            } else {
                normalize_whitespace(text)
            };
            // Skip whitespace-only spans unless we're at a block boundary
            // (they appear from newlines between tags and add noise)
            if t.is_empty() || (!style.in_pre && t == " ") {
                return;
            }
            push_inline(blocks, Span {
                text: t,
                bold: style.bold,
                italic: style.italic,
                underline: style.underline,
                strikethrough: style.strikethrough,
                link_index: style.link_index,
                fg: style.fg,
                bg: None,
            });
        }
        NodeData::Element { tag, attrs } => {
            // noscript: show content only when JS is disabled
            if tag == "noscript" {
                if !ctx.js_enabled {
                    for child in &node.children {
                        walk(child, ctx, blocks, style);
                    }
                }
                return;
            }

            // head is normally invisible but may contain <noscript> with fallback content
            if tag == "head" {
                for child in &node.children {
                    if child.tag() == Some("noscript") {
                        walk(child, ctx, blocks, style);
                    }
                }
                return;
            }

            let mut computed = css::default_style_for_tag(tag);

            // Apply inline style="" attribute overrides
            if let Some(style_attr) = attrs.get("style") {
                css::apply_inline_style(style_attr, &mut computed);
            }

            if computed.display == Display::None {
                return;
            }

            // Also respect hidden attribute
            if attrs.contains_key("hidden") {
                return;
            }

            let mut child_style = style.clone();
            if computed.font_weight == FontWeight::Bold {
                child_style.bold = true;
            }
            if computed.font_style == FontStyle::Italic {
                child_style.italic = true;
            }
            if computed.text_decoration == TextDecoration::Underline {
                child_style.underline = true;
            }
            if computed.text_decoration == TextDecoration::LineThrough {
                child_style.strikethrough = true;
            }
            if computed.white_space == WhiteSpace::Pre {
                child_style.in_pre = true;
            }
            if let Some(color) = computed.color {
                child_style.fg = Some(color);
            }
            // visibility:hidden → don't render children
            if computed.visibility == css::Visibility::Hidden {
                return;
            }

            match tag.as_str() {
                "html" | "body" | "div" | "section" | "article" | "main" | "header"
                | "footer" | "nav" | "aside" | "details" | "summary" | "address" | "center" => {
                    flush_to_paragraph(blocks);
                    for child in &node.children {
                        walk(child, ctx, blocks, &child_style);
                    }
                    flush_to_paragraph(blocks);
                }
                "p" => {
                    flush_to_paragraph(blocks);
                    for child in &node.children {
                        walk(child, ctx, blocks, &child_style);
                    }
                    flush_to_paragraph(blocks);
                    blocks.push(Block::Spacer);
                }
                "br" => {
                    push_inline(blocks, Span::plain("\n"));
                }
                "hr" => {
                    flush_to_paragraph(blocks);
                    blocks.push(Block::HRule);
                }
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    flush_to_paragraph(blocks);
                    blocks.push(Block::Spacer);
                    let level: u8 = tag[1..].parse().unwrap_or(1);
                    let mut child_blocks: Vec<Block> = Vec::new();
                    for child in &node.children {
                        walk(child, ctx, &mut child_blocks, &child_style);
                    }
                    // Collect inline spans from child_blocks
                    let mut heading_line: Inline = Vec::new();
                    for b in child_blocks {
                        match b {
                            Block::Paragraph(lines) => {
                                for line in lines {
                                    heading_line.extend(line);
                                }
                            }
                            _ => {}
                        }
                    }
                    blocks.push(Block::Heading {
                        level,
                        line: heading_line,
                    });
                    blocks.push(Block::Spacer);
                }
                "pre" | "code" | "kbd" | "samp" => {
                    if tag.as_str() == "pre" {
                        flush_to_paragraph(blocks);
                        let raw = node.text_content();
                        let lines: Vec<String> = raw.lines().map(String::from).collect();
                        blocks.push(Block::Pre(lines));
                    } else {
                        for child in &node.children {
                            walk(child, ctx, blocks, &child_style);
                        }
                    }
                }
                "blockquote" => {
                    flush_to_paragraph(blocks);
                    let mut inner = Vec::new();
                    for child in &node.children {
                        walk(child, ctx, &mut inner, &child_style);
                    }
                    flush_inner_paragraph(&mut inner);
                    blocks.push(Block::Blockquote(inner));
                }
                "ul" | "ol" => {
                    flush_to_paragraph(blocks);
                    let ordered = tag == "ol";
                    ctx.list_stack.push((ordered, 0));
                    for child in &node.children {
                        walk(child, ctx, blocks, &child_style);
                    }
                    ctx.list_stack.pop();
                }
                "li" => {
                    flush_to_paragraph(blocks);
                    let (ordered, index) = if let Some(entry) = ctx.list_stack.last_mut() {
                        entry.1 += 1;
                        (entry.0, entry.1)
                    } else {
                        (false, 1)
                    };

                    let mut item_blocks: Vec<Block> = Vec::new();
                    for child in &node.children {
                        walk(child, ctx, &mut item_blocks, &child_style);
                    }
                    let content = collect_inlines_from_blocks(item_blocks);
                    blocks.push(Block::ListItem {
                        ordered,
                        index,
                        content,
                    });
                }
                "table" => {
                    flush_to_paragraph(blocks);
                    for child in &node.children {
                        walk(child, ctx, blocks, &child_style);
                    }
                }
                "thead" | "tbody" | "tfoot" | "colgroup" | "col" => {
                    for child in &node.children {
                        walk(child, ctx, blocks, &child_style);
                    }
                }
                "tr" => {
                    let mut cells: Vec<Vec<Inline>> = Vec::new();
                    for child in &node.children {
                        if child.tag() == Some("td") || child.tag() == Some("th") {
                            let mut cell_blocks: Vec<Block> = Vec::new();
                            for grandchild in &child.children {
                                let mut cs = child_style.clone();
                                if child.tag() == Some("th") {
                                    cs.bold = true;
                                }
                                walk(grandchild, ctx, &mut cell_blocks, &cs);
                            }
                            cells.push(collect_inlines_from_blocks(cell_blocks));
                        }
                    }
                    if !cells.is_empty() {
                        blocks.push(Block::TableRow(cells));
                    }
                }
                "a" => {
                    let href = attrs.get("href").cloned().unwrap_or_default();
                    if !href.is_empty() && !href.starts_with("javascript:") {
                        let link_index = ctx.links.len();
                        ctx.links.push(Link {
                            text: node.text_content(),
                            url: href.clone(),
                        });
                        child_style.link_index = Some(link_index);
                        child_style.underline = true;
                    }
                    for child in &node.children {
                        walk(child, ctx, blocks, &child_style);
                    }
                }
                "img" => {
                    let alt = attrs.get("alt").cloned().unwrap_or_default();
                    if !alt.is_empty() {
                        let span = Span {
                            text: format!("[img: {}]", alt),
                            bold: false,
                            italic: true,
                            underline: false,
                            strikethrough: false,
                            link_index: style.link_index,
                            fg: None,
                            bg: None,
                        };
                        push_inline(blocks, span);
                    }
                }
                "form" => {
                    flush_to_paragraph(blocks);
                    let form_index = ctx.forms.len();
                    let action = attrs.get("action").cloned().unwrap_or_default();
                    let method = match attrs.get("method").map(|s| s.to_lowercase()).as_deref() {
                        Some("post") => FormMethod::Post,
                        _ => FormMethod::Get,
                    };
                    let enctype = attrs
                        .get("enctype")
                        .cloned()
                        .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string());
                    ctx.forms.push(Form { action, method, enctype });
                    ctx.form_stack.push(form_index);
                    for child in &node.children {
                        walk(child, ctx, blocks, &child_style);
                    }
                    ctx.form_stack.pop();
                    flush_to_paragraph(blocks);
                }
                "fieldset" | "legend" => {
                    flush_to_paragraph(blocks);
                    if tag == "legend" {
                        child_style.bold = true;
                    }
                    for child in &node.children {
                        walk(child, ctx, blocks, &child_style);
                    }
                    flush_to_paragraph(blocks);
                }
                "input" => {
                    let mut field = parse_input(attrs, ctx);
                    if field.field_type != FormFieldType::Hidden {
                        ctx.form_fields.push(field.clone());
                        blocks.push(Block::FormField(field));
                    } else {
                        ctx.form_fields.push(field);
                    }
                }
                "textarea" => {
                    let name = attrs.get("name").cloned().unwrap_or_default();
                    let default_value = node.text_content();
                    let field = FormField {
                        index: ctx.form_fields.len(),
                        form_index: ctx.form_stack.last().copied().unwrap_or(0),
                        field_type: FormFieldType::TextArea,
                        name,
                        default_value,
                        options: Vec::new(),
                    };
                    ctx.form_fields.push(field.clone());
                    blocks.push(Block::FormField(field));
                }
                "select" => {
                    let name = attrs.get("name").cloned().unwrap_or_default();
                    let mut options = Vec::new();
                    let mut default_value = String::new();
                    for opt in node.find_all("option") {
                        let val = opt.attr("value").unwrap_or("").to_string();
                        let label = opt.text_content().trim().to_string();
                        if opt.attr("selected").is_some() && default_value.is_empty() {
                            default_value = val.clone();
                        }
                        options.push((val, label));
                    }
                    if default_value.is_empty() {
                        if let Some((v, _)) = options.first() {
                            default_value = v.clone();
                        }
                    }
                    let field = FormField {
                        index: ctx.form_fields.len(),
                        form_index: ctx.form_stack.last().copied().unwrap_or(0),
                        field_type: FormFieldType::Select,
                        name,
                        default_value,
                        options,
                    };
                    ctx.form_fields.push(field.clone());
                    blocks.push(Block::FormField(field));
                }
                "dd" | "dt" => {
                    flush_to_paragraph(blocks);
                    let indent = if tag == "dd" { "    " } else { "" };
                    let mut child_blocks = Vec::new();
                    for child in &node.children {
                        walk(child, ctx, &mut child_blocks, &child_style);
                    }
                    let lines = collect_inlines_from_blocks(child_blocks);
                    for mut line in lines {
                        if !indent.is_empty() {
                            line.insert(0, Span::plain(indent));
                        }
                        blocks.push(Block::Paragraph(vec![line]));
                    }
                }
                _ => {
                    for child in &node.children {
                        walk(child, ctx, blocks, &child_style);
                    }
                }
            }
        }
        NodeData::Document | NodeData::Doctype | NodeData::Comment(_) => {
            for child in &node.children {
                walk(child, ctx, blocks, style);
            }
        }
    }
}

fn parse_input(attrs: &std::collections::HashMap<String, String>, ctx: &Ctx) -> FormField {
    let input_type = attrs.get("type").map(|s| s.to_lowercase());
    let input_type = input_type.as_deref().unwrap_or("text");
    let name = attrs.get("name").cloned().unwrap_or_default();
    let default_value = attrs.get("value").cloned().unwrap_or_default();

    let field_type = match input_type {
        "password" => FormFieldType::Password,
        "submit" | "button" => FormFieldType::Submit,
        "reset" => FormFieldType::Submit, // treat reset as submit for simplicity
        "checkbox" => FormFieldType::Checkbox {
            checked: attrs.contains_key("checked"),
        },
        "radio" => FormFieldType::Radio {
            checked: attrs.contains_key("checked"),
        },
        "hidden" => FormFieldType::Hidden,
        "email" | "search" | "tel" | "url" | "number" | "date" | "text" => FormFieldType::Text,
        _ => FormFieldType::Text,
    };

    FormField {
        index: ctx.form_fields.len(),
        form_index: ctx.form_stack.last().copied().unwrap_or(0),
        field_type,
        name,
        default_value,
        options: Vec::new(),
    }
}

/// Close the current inline context so the next push_inline starts a fresh Paragraph.
fn flush_to_paragraph(blocks: &mut Vec<Block>) {
    if matches!(blocks.last(), Some(Block::Paragraph(_))) {
        blocks.push(Block::Spacer);
    }
}

fn flush_inner_paragraph(blocks: &mut Vec<Block>) {
    // Ensure the inner block list ends cleanly.
    if let Some(Block::Paragraph(lines)) = blocks.last() {
        if lines.is_empty() {
            blocks.pop();
        }
    }
}

/// Push a span into the current pending Paragraph, creating one if needed.
fn push_inline(blocks: &mut Vec<Block>, span: Span) {
    match blocks.last_mut() {
        Some(Block::Paragraph(lines)) => {
            if span.text == "\n" {
                lines.push(Vec::new());
            } else {
                if lines.is_empty() {
                    lines.push(Vec::new());
                }
                lines.last_mut().unwrap().push(span);
            }
        }
        _ => {
            if span.text == "\n" {
                blocks.push(Block::Paragraph(vec![Vec::new()]));
            } else {
                blocks.push(Block::Paragraph(vec![vec![span]]));
            }
        }
    }
}

fn collect_inlines_from_blocks(blocks: Vec<Block>) -> Vec<Inline> {
    let mut result: Vec<Inline> = Vec::new();
    for block in blocks {
        match block {
            Block::Paragraph(lines) => result.extend(lines),
            Block::Heading { line, .. } => result.push(line),
            _ => {}
        }
    }
    result
}

fn normalize_whitespace(text: &str) -> String {
    // Collapse runs of whitespace to a single space, but preserve content.
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                out.push(' ');
            }
            last_was_space = true;
        } else {
            out.push(c);
            last_was_space = false;
        }
    }
    out
}
