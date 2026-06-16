use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span as RSpan, Text},
};
use unicode_width::UnicodeWidthStr;

use crate::layout::{Block, FormField, FormFieldType, Link, Span};

/// Convert layout blocks into ratatui Text for display.
pub fn render(
    blocks: &[Block],
    links: &[Link],
    width: u16,
    focused_link: Option<usize>,
) -> Text<'static> {
    let width = width as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();

    for block in blocks {
        render_block(block, links, width, focused_link, &mut lines);
    }

    Text::from(lines)
}

fn render_block(
    block: &Block,
    links: &[Link],
    width: usize,
    focused_link: Option<usize>,
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
                    let wrapped = wrap_inline(inline, links, width, focused_link);
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
            let full = format!("{}{}", prefix, text);
            out.push(Line::from(RSpan::styled(
                full,
                Style::default()
                    .fg(color)
                    .add_modifier(Modifier::BOLD),
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
                render_block(b, links, width.saturating_sub(2), focused_link, &mut inner_lines);
            }
            for line in inner_lines {
                let mut spans = vec![RSpan::styled(
                    "▌ ",
                    Style::default().fg(Color::DarkGray),
                )];
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
            let wrapped = wrap_inline(&all_spans, links, available, focused_link);

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
            let mut row_spans = vec![RSpan::styled(
                "│",
                Style::default().fg(Color::DarkGray),
            )];
            for cell_inlines in cells {
                let all_spans: Vec<Span> = cell_inlines.iter().flatten().cloned().collect();
                let text = spans_to_string(&all_spans);
                let text = truncate_to_width(&text, cell_width.saturating_sub(3));
                let padded = format!(" {:<width$} ", text, width = cell_width.saturating_sub(3));
                row_spans.push(RSpan::raw(padded));
                row_spans.push(RSpan::styled(
                    "│",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            out.push(Line::from(row_spans));
        }
        Block::FormField(field) => {
            render_form_field(field, width, out);
        }
    }
}

fn render_form_field(field: &FormField, width: usize, out: &mut Vec<Line<'static>>) {
    match &field.field_type {
        FormFieldType::Submit => {
            let label = if field.value.is_empty() {
                "Submit"
            } else {
                &field.value
            };
            out.push(Line::from(vec![
                RSpan::raw("  "),
                RSpan::styled(
                    format!("[ {} ]", label),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
        FormFieldType::Text | FormFieldType::Password => {
            let display_value = if field.field_type == FormFieldType::Password {
                "*".repeat(field.value.len())
            } else {
                field.value.clone()
            };
            let box_width = width.saturating_sub(20).max(20);
            out.push(Line::from(vec![
                RSpan::styled(
                    format!("  {:>16}: ", field.name),
                    Style::default().fg(Color::Cyan),
                ),
                RSpan::styled(
                    format!("[{:<width$}]", display_value, width = box_width),
                    Style::default().fg(Color::White).bg(Color::DarkGray),
                ),
            ]));
        }
        FormFieldType::Checkbox { checked } => {
            out.push(Line::from(vec![
                RSpan::raw("  "),
                RSpan::styled(
                    if *checked { "[✓] " } else { "[ ] " },
                    Style::default().fg(Color::Green),
                ),
                RSpan::raw(field.name.clone()),
            ]));
        }
        FormFieldType::Radio { checked } => {
            out.push(Line::from(vec![
                RSpan::raw("  "),
                RSpan::styled(
                    if *checked { "(●) " } else { "( ) " },
                    Style::default().fg(Color::Green),
                ),
                RSpan::raw(field.name.clone()),
            ]));
        }
        FormFieldType::Select => {
            let current = field
                .options
                .first()
                .map(|(_, l)| l.as_str())
                .unwrap_or("");
            out.push(Line::from(vec![
                RSpan::styled(
                    format!("  {:>16}: ", field.name),
                    Style::default().fg(Color::Cyan),
                ),
                RSpan::styled(
                    format!("[{} ▼]", current),
                    Style::default().fg(Color::White).bg(Color::DarkGray),
                ),
            ]));
        }
        FormFieldType::TextArea => {
            out.push(Line::from(RSpan::styled(
                format!("  [TextArea: {}]", field.name),
                Style::default().fg(Color::Cyan),
            )));
        }
        FormFieldType::Hidden => {}
    }
}

#[allow(dead_code)]
fn span_to_ratatui(span: &Span, _links: &[Link], focused_link: Option<usize>) -> RSpan<'static> {
    let mut style = Style::default();

    if span.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if span.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if span.underline {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if span.strikethrough {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }
    if let Some(fg) = span.fg {
        style = style.fg(Color::Rgb(fg.r, fg.g, fg.b));
    }
    if let Some(idx) = span.link_index {
        let is_focused = focused_link == Some(idx);
        if is_focused {
            style = style
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
        } else {
            style = style.fg(Color::Cyan).add_modifier(Modifier::UNDERLINED);
        }
    }

    RSpan::styled(span.text.clone(), style)
}

/// Wrap a slice of inline spans to a given terminal width.
fn wrap_inline(
    spans: &[Span],
    links: &[Link],
    width: usize,
    focused_link: Option<usize>,
) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![];
    }

    // Flatten spans into word units preserving style boundaries
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<RSpan<'static>> = Vec::new();
    let mut current_width = 0usize;

    for span in spans {
        let text = &span.text;
        let style = span_style(span, links, focused_link);

        // Split on spaces to word-wrap
        let mut words = text.split_inclusive(' ').peekable();
        while let Some(word) = words.next() {
            let word_width = UnicodeWidthStr::width(word);

            if current_width + word_width > width && current_width > 0 {
                // Flush line
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

fn span_style(span: &Span, _links: &[Link], focused_link: Option<usize>) -> Style {
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
        let is_focused = focused_link == Some(idx);
        if is_focused {
            style = style.fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD);
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
