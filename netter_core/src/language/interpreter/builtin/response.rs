use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub is_sent: bool,
}

impl Response {
    pub fn new() -> Self {
        Response {
            status: 200,
            headers: HashMap::new(),
            body: None,
            is_sent: false,
        }
    }

    pub fn body(&mut self, content: impl Into<String>) -> &mut Self {
        self.body = Some(content.into());
        self
    }

    pub fn send(&mut self) {
        self.is_sent = true;
        if !self.headers.contains_key("Content-Type") && self.body.is_some() {
            self.headers.insert(
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            );
        }
    }

    pub fn status(&mut self, status: u16) -> &mut Self {
        self.status = status;
        self
    }

    pub fn set_header(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn is_sent(&self) -> bool {
        self.is_sent
    }
}