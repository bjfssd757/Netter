use std::collections::HashMap;
use base64::Engine;

#[derive(Debug, Clone)]
pub enum HttpBodyVariant {
    Text(String),
    Bytes(Vec<u8>),
    Empty,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: HttpBodyVariant,
}

impl Request {
    pub fn new(params: HashMap<String, String>, headers: HashMap<String, String>, body: HttpBodyVariant) -> Self {
        Request {
            params,
            headers,
            body,
        }
    }

    pub fn empty() -> Self {
        Request {
            params: HashMap::new(),
            headers: HashMap::new(),
            body: HttpBodyVariant::Empty,
        }
    }

    pub fn get_param(&self, name: &str) -> String {
        self.params.get(name).cloned().unwrap_or_default()
    }

    pub fn get_header(&self, name: &str) -> String {
        self.headers.get(name).cloned().unwrap_or_default()
    }

    pub fn get_body(&self) -> String {
        match &self.body {
            HttpBodyVariant::Empty => "".to_string(),
            HttpBodyVariant::Text(text) => text.clone(),
            HttpBodyVariant::Bytes(_) => "[Binary Body - Use body_base64() for content]".to_string(),
        }
    }

    pub fn get_body_as_base64(&self) -> String {
        match &self.body {
            HttpBodyVariant::Text(s) => base64::engine::general_purpose::STANDARD.encode(s.as_bytes()),
            HttpBodyVariant::Bytes(bytes_vec) => base64::engine::general_purpose::STANDARD.encode(bytes_vec),
            HttpBodyVariant::Empty => "".to_string(),
        }
    }
    
    pub fn is_body_binary(&self) -> bool {
        matches!(&self.body, HttpBodyVariant::Bytes(_))
    }
}