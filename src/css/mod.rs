/// Computed style properties relevant to terminal rendering.
#[derive(Debug, Clone, Default)]
pub struct ComputedStyle {
    pub display: Display,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub text_decoration: TextDecoration,
    pub color: Option<TermColor>,
    pub background_color: Option<TermColor>,
    pub visibility: Visibility,
    pub white_space: WhiteSpace,
    pub list_style_type: ListStyleType,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Display {
    #[default]
    Inline,
    Block,
    None,
    ListItem,
    TableCell,
    TableRow,
    Table,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    LineThrough,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum WhiteSpace {
    #[default]
    Normal,
    Pre,
    PreWrap,
    NoWrap,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ListStyleType {
    #[default]
    Disc,
    Decimal,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TermColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl TermColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// Return the default computed style for a given HTML tag name.
pub fn default_style_for_tag(tag: &str) -> ComputedStyle {
    let mut s = ComputedStyle::default();
    match tag {
        "html" | "body" | "div" | "p" | "section" | "article" | "main" | "header"
        | "footer" | "nav" | "aside" | "figure" | "figcaption" | "blockquote" | "details"
        | "summary" | "dl" | "dt" | "dd" | "address" | "form" | "fieldset" | "legend"
        | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            s.display = Display::Block;
        }
        "ul" | "ol" => {
            s.display = Display::Block;
        }
        "li" => {
            s.display = Display::ListItem;
        }
        "table" => {
            s.display = Display::Table;
        }
        "tr" => {
            s.display = Display::TableRow;
        }
        "td" | "th" => {
            s.display = Display::TableCell;
        }
        "script" | "style" | "meta" | "link" | "head" | "noscript" | "template" => {
            s.display = Display::None;
        }
        "strong" | "b" => {
            s.font_weight = FontWeight::Bold;
        }
        "em" | "i" | "cite" | "var" => {
            s.font_style = FontStyle::Italic;
        }
        "u" | "ins" => {
            s.text_decoration = TextDecoration::Underline;
        }
        "s" | "del" | "strike" => {
            s.text_decoration = TextDecoration::LineThrough;
        }
        "pre" | "code" | "kbd" | "samp" | "tt" => {
            s.white_space = WhiteSpace::Pre;
        }
        _ => {}
    }
    match tag {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            s.font_weight = FontWeight::Bold;
        }
        _ => {}
    }
    s
}
