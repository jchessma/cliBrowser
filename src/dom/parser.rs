use std::collections::HashMap;

use scraper::{Html, Node as ScraperNode};

use super::node::{Document, Node, NodeData};

pub fn parse_html(html: &str) -> Document {
    let document = Html::parse_document(html);
    let root_node = convert_tree_node(document.tree.root());
    Document::new(root_node)
}

fn convert_tree_node(node: ego_tree::NodeRef<scraper::Node>) -> Node {
    match node.value() {
        ScraperNode::Document => {
            let mut root = Node::new(NodeData::Document);
            for child in node.children() {
                root.children.push(convert_tree_node(child));
            }
            root
        }
        ScraperNode::Element(el) => {
            let tag = el.name().to_string().to_lowercase();
            let attrs: HashMap<String, String> = el
                .attrs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let mut elem = Node::new(NodeData::Element { tag, attrs });
            for child in node.children() {
                elem.children.push(convert_tree_node(child));
            }
            elem
        }
        ScraperNode::Text(text) => Node::new(NodeData::Text(text.text.to_string())),
        ScraperNode::Comment(c) => Node::new(NodeData::Comment(c.comment.to_string())),
        ScraperNode::Doctype(_) => Node::new(NodeData::Doctype),
        ScraperNode::ProcessingInstruction(_) => Node::new(NodeData::Comment(String::new())),
        ScraperNode::Fragment => {
            let mut root = Node::new(NodeData::Document);
            for child in node.children() {
                root.children.push(convert_tree_node(child));
            }
            root
        }
    }
}
