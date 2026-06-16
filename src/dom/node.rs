use std::collections::HashMap;

pub type Attr = HashMap<String, String>;

#[derive(Debug, Clone)]
pub enum NodeData {
    Document,
    Element {
        tag: String,
        attrs: Attr,
    },
    Text(String),
    Comment(String),
    Doctype,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub data: NodeData,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(data: NodeData) -> Self {
        Self {
            data,
            children: Vec::new(),
        }
    }

    pub fn tag(&self) -> Option<&str> {
        match &self.data {
            NodeData::Element { tag, .. } => Some(tag.as_str()),
            _ => None,
        }
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        match &self.data {
            NodeData::Element { attrs, .. } => attrs.get(name).map(|s| s.as_str()),
            _ => None,
        }
    }

    pub fn text_content(&self) -> String {
        match &self.data {
            NodeData::Text(t) => t.clone(),
            _ => self.children.iter().map(|c| c.text_content()).collect(),
        }
    }

    pub fn find_all<'a>(&'a self, tag: &str) -> Vec<&'a Node> {
        let mut results = Vec::new();
        self.collect_by_tag(tag, &mut results);
        results
    }

    fn collect_by_tag<'a>(&'a self, tag: &str, out: &mut Vec<&'a Node>) {
        if self.tag() == Some(tag) {
            out.push(self);
        }
        for child in &self.children {
            child.collect_by_tag(tag, out);
        }
    }

    pub fn find_first(&self, tag: &str) -> Option<&Node> {
        if self.tag() == Some(tag) {
            return Some(self);
        }
        for child in &self.children {
            if let Some(n) = child.find_first(tag) {
                return Some(n);
            }
        }
        None
    }
}

pub struct Document {
    pub root: Node,
    pub title: String,
}

impl Document {
    pub fn new(root: Node) -> Self {
        let title = root
            .find_first("title")
            .map(|n| n.text_content().trim().to_string())
            .unwrap_or_default();
        Self { root, title }
    }
}
