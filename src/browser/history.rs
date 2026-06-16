/// Simple back/forward navigation stack.
pub struct History {
    back: Vec<String>,
    current: Option<String>,
    forward: Vec<String>,
}

impl History {
    pub fn new() -> Self {
        Self {
            back: Vec::new(),
            current: None,
            forward: Vec::new(),
        }
    }

    pub fn navigate(&mut self, url: String) {
        if let Some(cur) = self.current.take() {
            self.back.push(cur);
        }
        self.current = Some(url);
        self.forward.clear();
    }

    pub fn back(&mut self) -> Option<String> {
        let prev = self.back.pop()?;
        if let Some(cur) = self.current.take() {
            self.forward.push(cur);
        }
        self.current = Some(prev.clone());
        Some(prev)
    }

    pub fn forward(&mut self) -> Option<String> {
        let next = self.forward.pop()?;
        if let Some(cur) = self.current.take() {
            self.back.push(cur);
        }
        self.current = Some(next.clone());
        Some(next)
    }

    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }
}
