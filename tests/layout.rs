use clibrowser::dom::parse_html;
use clibrowser::layout::{layout_with_opts, Block, FormFieldType, TabItem};

fn blocks_for(html: &str) -> Vec<Block> {
    let doc = parse_html(html);
    layout_with_opts(&doc.root, false).blocks
}

fn all_text(blocks: &[Block]) -> Vec<String> {
    let mut out = Vec::new();
    for block in blocks {
        match block {
            Block::Paragraph(lines) => {
                for line in lines {
                    let s: String = line.iter().map(|sp| sp.text.as_str()).collect();
                    if !s.trim().is_empty() {
                        out.push(s.trim().to_string());
                    }
                }
            }
            Block::Heading { line, .. } => {
                let s: String = line.iter().map(|sp| sp.text.as_str()).collect();
                if !s.trim().is_empty() {
                    out.push(s.trim().to_string());
                }
            }
            _ => {}
        }
    }
    out
}

#[test]
fn adjacent_divs_do_not_merge() {
    let html = "<div>Hello</div><div>World</div>";
    let texts = all_text(&blocks_for(html));
    assert!(texts.len() >= 2, "Expected 2 blocks, got: {:?}", texts);
    assert!(texts[0].contains("Hello"), "First block should contain Hello: {:?}", texts);
    assert!(texts[1].contains("World"), "Second block should contain World: {:?}", texts);
}

#[test]
fn paragraphs_are_separate() {
    let html = "<p>First</p><p>Second</p><p>Third</p>";
    let texts = all_text(&blocks_for(html));
    assert_eq!(texts.len(), 3, "Expected 3 paragraphs, got: {:?}", texts);
}

#[test]
fn heading_renders_bold() {
    let html = "<h1>Main Title</h1><p>Body text</p>";
    let doc = parse_html(html);
    let result = layout_with_opts(&doc.root, false);
    let heading = result.blocks.iter().find(|b| matches!(b, Block::Heading { level: 1, .. }));
    assert!(heading.is_some(), "H1 should produce a Heading block");
    if let Some(Block::Heading { line, .. }) = heading {
        let text: String = line.iter().map(|s| s.text.as_str()).collect();
        assert!(text.contains("Main Title"));
        assert!(line.iter().all(|s| s.bold), "H1 spans should be bold");
    }
}

#[test]
fn links_are_collected() {
    let html = r#"<a href="https://example.com">Example</a> and <a href="/page">Page</a>"#;
    let doc = parse_html(html);
    let result = layout_with_opts(&doc.root, false);
    assert_eq!(result.links.len(), 2);
    assert_eq!(result.links[0].url, "https://example.com");
    assert_eq!(result.links[1].url, "/page");
}

#[test]
fn display_none_hides_element() {
    let html = r#"<div>Visible</div><div style="display:none">Hidden</div><div>Also Visible</div>"#;
    let texts = all_text(&blocks_for(html));
    assert!(texts.iter().any(|t| t.contains("Visible")));
    assert!(!texts.iter().any(|t| t.contains("Hidden")), "display:none element should be hidden");
}

#[test]
fn noscript_shown_when_js_disabled() {
    let html = "<noscript>Please enable JavaScript</noscript><div>Main content</div>";
    let doc = parse_html(html);
    let result = layout_with_opts(&doc.root, false); // JS disabled
    let texts = all_text(&result.blocks);
    assert!(
        texts.iter().any(|t| t.contains("Please enable JavaScript")),
        "noscript content should show when JS is disabled: {:?}", texts
    );
}

#[test]
fn noscript_hidden_when_js_enabled() {
    let html = "<noscript>Please enable JavaScript</noscript><div>Main content</div>";
    let doc = parse_html(html);
    let result = layout_with_opts(&doc.root, true); // JS enabled
    let texts = all_text(&result.blocks);
    assert!(
        !texts.iter().any(|t| t.contains("Please enable JavaScript")),
        "noscript content should be hidden when JS is enabled: {:?}", texts
    );
}

#[test]
fn list_items_have_bullets() {
    let html = "<ul><li>Alpha</li><li>Beta</li></ul>";
    let doc = parse_html(html);
    let result = layout_with_opts(&doc.root, false);
    let list_items: Vec<&Block> = result
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::ListItem { ordered: false, .. }))
        .collect();
    assert_eq!(list_items.len(), 2, "Expected 2 unordered list items");
}

#[test]
fn pre_block_preserves_whitespace() {
    let html = "<pre>  line 1\n    line 2\n      line 3</pre>";
    let doc = parse_html(html);
    let result = layout_with_opts(&doc.root, false);
    let pre = result.blocks.iter().find(|b| matches!(b, Block::Pre(_)));
    assert!(pre.is_some(), "Expected a Pre block");
    if let Some(Block::Pre(lines)) = pre {
        assert!(lines[0].starts_with("  line 1"), "Indentation should be preserved");
        assert!(lines[1].starts_with("    line 2"));
    }
}

#[test]
fn inline_bold_applies_to_spans() {
    let html = "<p>Normal <strong>bold</strong> normal</p>";
    let doc = parse_html(html);
    let result = layout_with_opts(&doc.root, false);
    let mut found_bold = false;
    for block in &result.blocks {
        if let Block::Paragraph(lines) = block {
            for line in lines {
                for span in line {
                    if span.text.contains("bold") {
                        assert!(span.bold, "span containing 'bold' should have bold=true");
                        found_bold = true;
                    }
                    if span.text.trim() == "Normal" {
                        assert!(!span.bold, "Normal text should not be bold");
                    }
                }
            }
        }
    }
    assert!(found_bold, "Should have found a bold span");
}

#[test]
fn form_fields_in_tab_order() {
    let html = r#"
        <form action="/search" method="get">
            <a href="/home">Home</a>
            <input type="text" name="q" value="">
            <a href="/about">About</a>
            <input type="submit" value="Search">
        </form>
    "#;
    let doc = parse_html(html);
    let result = layout_with_opts(&doc.root, true);

    // Should find 2 links and 2 form fields (text + submit)
    assert_eq!(result.links.len(), 2, "Should have 2 links");
    assert_eq!(result.form_fields.len(), 2, "Should have 2 form fields (text + submit)");

    // Tab order: Home link, text input, About link, Submit button
    assert_eq!(result.tab_order.len(), 4);
    assert!(matches!(result.tab_order[0], TabItem::Link(0)));
    assert!(matches!(result.tab_order[1], TabItem::Field(0)));
    assert!(matches!(result.tab_order[2], TabItem::Link(1)));
    assert!(matches!(result.tab_order[3], TabItem::Field(1)));

    // Text field should be type Text, submit should be Submit
    assert_eq!(result.form_fields[0].field_type, FormFieldType::Text);
    assert_eq!(result.form_fields[1].field_type, FormFieldType::Submit);
}

#[test]
fn form_field_name_and_action() {
    let html = r#"<form action="/search" method="get"><input type="search" name="q"></form>"#;
    let doc = parse_html(html);
    let result = layout_with_opts(&doc.root, true);

    assert_eq!(result.forms.len(), 1);
    assert_eq!(result.forms[0].action, "/search");
    assert_eq!(result.form_fields.len(), 1);
    assert_eq!(result.form_fields[0].name, "q");
    assert_eq!(result.form_fields[0].field_type, FormFieldType::Text);
}
