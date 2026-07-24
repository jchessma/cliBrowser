use std::collections::HashMap;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span as RSpan, Text},
};
use unicode_width::UnicodeWidthStr;

use crate::layout::{Block, FormField, FormFieldType, Link, Span};

pub struct PageFocus<'a> {
    /// Focused link index (raw index into `links` vec).
    pub link: Option<usize>,
    /// Focused form field index (FormField::index).
    pub field: Option<usize>,
    /// Whether the focused field is in active text-edit mode.
    pub editing: bool,
    /// Current live values for form fields.
    pub field_values: &'a HashMap<usize, String>,
}

/// Convert layout blocks into ratatui Text for display.
pub fn render(
    blocks: &[Block],
    links: &[Link],
    width: u16,
    focus: &PageFocus<'_>,
) -> Text<'static> {
    let width = width as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();

    for block in blocks {
        render_block(block, links, width, focus, &mut lines);
    }

    Text::from(lines)
}

fn render_block(
    block: &Block,
    links: &[Link],
    width: usize,
    focus: &PageFocus<'_>,
    out: &mut Vec<Line<'static>>,
) {
    match block {
        Block::Spacer => {
            out.push(Line::raw(""));
        }
        Block::HRule => {
            out.push(Line::from(RSpan::styled(
                "─".repeat(width),
                Style::default().fg(Color::DarkGray),
            )));
        }
        Block::Paragraph(inline_lines) => {
            for inline in inline_lines {
                if inline.is_empty() {
                    out.push(Line::raw(""));
                } else {
                    let wrapped = wrap_inline(inline, width, focus.link);
                    out.extend(wrapped);
                }
            }
        }
        Block::Heading { level, line } => {
            let prefix = match level {
                1 => "# ",
                2 => "## ",
                3 => "### ",
                4 => "#### ",
                _ => "###### ",
            };
            let color = match level {
                1 => Color::Cyan,
                2 => Color::Blue,
                3 => Color::Green,
                4 => Color::Yellow,
                _ => Color::White,
            };
            let text = spans_to_string(line);
            out.push(Line::from(RSpan::styled(
                format!("{}{}", prefix, text),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )));
        }
        Block::Pre(code_lines) => {
            out.push(Line::from(RSpan::styled(
                "╔".to_string() + &"═".repeat(width.saturating_sub(1)),
                Style::default().fg(Color::DarkGray),
            )));
            for code_line in code_lines {
                out.push(Line::from(RSpan::styled(
                    format!("│ {}", code_line),
                    Style::default().fg(Color::Yellow),
                )));
            }
            out.push(Line::from(RSpan::styled(
                "╚".to_string() + &"═".repeat(width.saturating_sub(1)),
                Style::default().fg(Color::DarkGray),
            )));
        }
        Block::Blockquote(inner) => {
            let mut inner_lines: Vec<Line<'static>> = Vec::new();
            for b in inner {
                render_block(b, links, width.saturating_sub(2), focus, &mut inner_lines);
            }
            for line in inner_lines {
                let mut spans = vec![RSpan::styled("▌ ", Style::default().fg(Color::DarkGray))];
                spans.extend(line.spans);
                out.push(Line::from(spans));
            }
        }
        Block::ListItem {
            ordered,
            index,
            content,
        } => {
            let bullet = if *ordered {
                format!("  {}. ", index)
            } else {
                "  • ".to_string()
            };
            let indent = " ".repeat(bullet.len());
            let available = width.saturating_sub(bullet.len());

            let all_spans: Vec<Span> = content.iter().flatten().cloned().collect();
            let wrapped = wrap_inline(&all_spans, available, focus.link);

            for (i, line) in wrapped.into_iter().enumerate() {
                let prefix = if i == 0 {
                    RSpan::styled(bullet.clone(), Style::default().fg(Color::Cyan))
                } else {
                    RSpan::raw(indent.clone())
                };
                let mut spans = vec![prefix];
                spans.extend(line.spans);
                out.push(Line::from(spans));
            }
        }
        Block::TableRow(cells) => {
            let cell_width = if cells.is_empty() {
                width
            } else {
                width / cells.len()
            };
            let mut row_spans = vec![RSpan::styled("│", Style::default().fg(Color::DarkGray))];
            for cell_inlines in cells {
                let all_spans: Vec<Span> = cell_inlines.iter().flatten().cloned().collect();
                let text = spans_to_string(&all_spans);
                let text = truncate_to_width(&text, cell_width.saturating_sub(3));
                let padded = format!(" {:<width$} ", text, width = cell_width.saturating_sub(3));
                row_spans.push(RSpan::raw(padded));
                row_spans.push(RSpan::styled("│", Style::default().fg(Color::DarkGray)));
            }
            out.push(Line::from(row_spans));
        }
        Block::FormField(field) => {
            render_form_field(field, focus, width, out);
        }
    }
}

fn render_form_field(
    field: &FormField,
    focus: &PageFocus<'_>,
    width: usize,
    out: &mut Vec<Line<'static>>,
) {
    let is_focused = focus.field == Some(field.index);
    let is_editing = is_focused && focus.editing;

    let current_value = focus
        .field_values
        .get(&field.index)
        .map(|s| s.as_str())
        .unwrap_or(&field.default_value);

    match &field.field_type {
        FormFieldType::Submit => {
            let label = if current_value.is_empty() {
                "Submit"
            } else {
                current_value
            };
            let style = if is_focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            };
            out.push(Line::from(vec![
                RSpan::raw("  "),
                RSpan::styled(format!("[ {} ]", label), style),
                if is_focused {
                    RSpan::styled(" ← Enter to submit", Style::default().fg(Color::DarkGray))
                } else {
                    RSpan::raw("")
                },
            ]));
        }
        FormFieldType::Text | FormFieldType::Password | FormFieldType::TextArea => {
            let display_value = if field.field_type == FormFieldType::Password {
                "•".repeat(current_value.len())
            } else {
                current_value.to_string()
            };
            let cursor = if is_editing { "▌" } else { "" };
            let box_width = width.saturating_sub(22).max(20);
            let label_style = if is_focused {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            let field_style = if is_editing {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else if is_focused {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            };
            out.push(Line::from(vec![
                RSpan::styled(format!("  {:>16}: ", field.name), label_style),
                RSpan::styled(
                    format!(
                        "[{:<width$}{}]",
                        truncate_to_width(&display_value, box_width.saturating_sub(1)),
                        cursor,
                        width = box_width
                    ),
                    field_style,
                ),
            ]));
        }
        FormFieldType::Checkbox { checked } => {
            let is_checked = focus
                .field_values
                .get(&field.index)
                .map(|v| v == "on")
                .unwrap_or(*checked);
            let box_str = if is_checked { "[✓]" } else { "[ ]" };
            let style = if is_focused {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            out.push(Line::from(vec![
                RSpan::raw("  "),
                RSpan::styled(format!("{} ", box_str), style),
                RSpan::styled(
                    field.name.clone(),
                    if is_focused {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                if is_focused {
                    RSpan::styled(" ← Space to toggle", Style::default().fg(Color::DarkGray))
                } else {
                    RSpan::raw("")
                },
            ]));
        }
        FormFieldType::Radio { checked } => {
            let is_checked = focus
                .field_values
                .get(&field.index)
                .map(|v| v == "on")
                .unwrap_or(*checked);
            let box_str = if is_checked { "(●)" } else { "( )" };
            let style = if is_focused {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            out.push(Line::from(vec![
                RSpan::raw("  "),
                RSpan::styled(format!("{} ", box_str), style),
                RSpan::raw(field.name.clone()),
            ]));
        }
        FormFieldType::Select => {
            let current_label = field
                .options
                .iter()
                .find(|(v, _)| v == current_value)
                .map(|(_, l)| l.as_str())
                .unwrap_or(current_value);
            let label_style = if is_focused {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            out.push(Line::from(vec![
                RSpan::styled(format!("  {:>16}: ", field.name), label_style),
                RSpan::styled(
                    format!("[{} ▼]", current_label),
                    if is_focused {
                        Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White).bg(Color::DarkGray)
                    },
                ),
            ]));
        }
        FormFieldType::Hidden => {}
    }
}

/// Wrap a slice of inline spans to a given terminal width.
fn wrap_inline(
    spans: &[Span],
    width: usize,
    focused_link: Option<usize>,
) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![];
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<RSpan<'static>> = Vec::new();
    let mut current_width = 0usize;

    for span in spans {
        let style = span_style(span, focused_link);

        // Split on spaces to word-wrap, keeping the trailing space with the word
        let words: Vec<&str> = span.text.split_inclusive(' ').collect();
        for word in words {
            let word_width = UnicodeWidthStr::width(word);

            if current_width + word_width > width && current_width > 0 {
                lines.push(Line::from(std::mem::take(&mut current_line)));
                current_width = 0;
            }

            current_line.push(RSpan::styled(word.to_string(), style));
            current_width += word_width;
        }
    }

    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    if lines.is_empty() {
        lines.push(Line::raw(""));
    }

    lines
}

fn span_style(span: &Span, focused_link: Option<usize>) -> Style {
    let mut style = Style::default();
    if span.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if span.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if span.underline || span.link_index.is_some() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if span.strikethrough {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }
    if let Some(fg) = span.fg {
        style = style.fg(Color::Rgb(fg.r, fg.g, fg.b));
    }
    if let Some(idx) = span.link_index {
        if focused_link == Some(idx) {
            style = style
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
        } else {
            style = style.fg(Color::Cyan);
        }
    }
    style
}

fn spans_to_string(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

fn truncate_to_width(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = UnicodeWidthStr::width(c.to_string().as_str());
        if w + cw > max {
            break;
        }
        out.push(c);
        w += cw;
    }
    out
}

#[allow(dead_code)]
fn span_to_ratatui(span: &Span, _links: &[Link], focused_link: Option<usize>) -> RSpan<'static> {
    RSpan::styled(span.text.clone(), span_style(span, focused_link))
}
