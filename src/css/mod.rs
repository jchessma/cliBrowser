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

/// Apply CSS property declarations from a `style=""` attribute to an existing computed style.
/// Handles the most visually impactful properties for terminal rendering.
pub fn apply_inline_style(style_attr: &str, computed: &mut ComputedStyle) {
    for decl in style_attr.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        let Some((prop, value)) = decl.split_once(':') else {
            continue;
        };
        let prop = prop.trim().to_lowercase();
        let value = value.trim().to_lowercase();

        match prop.as_str() {
            "display" => match value.as_str() {
                "none" => computed.display = Display::None,
                "block" => computed.display = Display::Block,
                "inline" => computed.display = Display::Inline,
                "list-item" => computed.display = Display::ListItem,
                _ => {}
            },
            "visibility" => {
                if value == "hidden" || value == "collapse" {
                    computed.visibility = Visibility::Hidden;
                }
            }
            "font-weight" => match value.as_str() {
                "bold" | "bolder" | "700" | "800" | "900" => {
                    computed.font_weight = FontWeight::Bold;
                }
                "normal" | "400" | "300" | "200" | "100" => {
                    computed.font_weight = FontWeight::Normal;
                }
                _ => {
                    if let Ok(n) = value.parse::<u32>() {
                        if n >= 600 {
                            computed.font_weight = FontWeight::Bold;
                        }
                    }
                }
            },
            "font-style" => match value.as_str() {
                "italic" | "oblique" => computed.font_style = FontStyle::Italic,
                "normal" => computed.font_style = FontStyle::Normal,
                _ => {}
            },
            "text-decoration" | "text-decoration-line" => {
                if value.contains("underline") {
                    computed.text_decoration = TextDecoration::Underline;
                } else if value.contains("line-through") {
                    computed.text_decoration = TextDecoration::LineThrough;
                } else if value == "none" {
                    computed.text_decoration = TextDecoration::None;
                }
            }
            "color" => {
                if let Some(color) = parse_css_color(&value) {
                    computed.color = Some(color);
                }
            }
            "background-color" | "background" => {
                if let Some(color) = parse_css_color(&value) {
                    computed.background_color = Some(color);
                }
            }
            "white-space" => match value.as_str() {
                "pre" => computed.white_space = WhiteSpace::Pre,
                "pre-wrap" => computed.white_space = WhiteSpace::PreWrap,
                "nowrap" | "no-wrap" => computed.white_space = WhiteSpace::NoWrap,
                "normal" => computed.white_space = WhiteSpace::Normal,
                _ => {}
            },
            _ => {}
        }
    }
}

/// Parse a CSS color value to an RGB triple for terminal output.
fn parse_css_color(value: &str) -> Option<TermColor> {
    let v = value.trim();

    // Named colors (most common)
    let named = match v {
        "red" => Some((220, 50, 47)),
        "green" => Some((0, 128, 0)),
        "blue" => Some((0, 0, 255)),
        "yellow" => Some((255, 220, 0)),
        "orange" => Some((255, 165, 0)),
        "purple" | "violet" => Some((128, 0, 128)),
        "pink" => Some((255, 105, 180)),
        "gray" | "grey" => Some((128, 128, 128)),
        "black" => Some((0, 0, 0)),
        "white" => Some((255, 255, 255)),
        "cyan" | "aqua" => Some((0, 180, 180)),
        "magenta" | "fuchsia" => Some((180, 0, 180)),
        "darkred" => Some((139, 0, 0)),
        "darkgreen" => Some((0, 100, 0)),
        "darkblue" => Some((0, 0, 139)),
        "lightblue" => Some((173, 216, 230)),
        "lightgreen" => Some((144, 238, 144)),
        "lightyellow" => Some((255, 255, 224)),
        "coral" => Some((255, 127, 80)),
        "salmon" => Some((250, 128, 114)),
        "brown" => Some((139, 69, 19)),
        "teal" => Some((0, 128, 128)),
        "navy" => Some((0, 0, 128)),
        "transparent" => return None,
        _ => None,
    };
    if let Some((r, g, b)) = named {
        return Some(TermColor { r, g, b });
    }

    // #rgb or #rrggbb
    if let Some(hex) = v.strip_prefix('#') {
        return match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                Some(TermColor { r, g, b })
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(TermColor { r, g, b })
            }
            _ => None,
        };
    }

    // rgb(r, g, b)
    if let Some(inner) = v.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            return Some(TermColor { r, g, b });
        }
    }

    // rgba(r, g, b, a) — ignore alpha
    if let Some(inner) = v.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() >= 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            return Some(TermColor { r, g, b });
        }
    }

    None
}
