use std::collections::HashMap;
use base64::Engine;
use crate::language::rdl_types::RDLTypes;

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

    pub fn get_param(&self, name: &RDLTypes) -> RDLTypes {
        RDLTypes::String(self.params.get(name.to_string().as_str()).cloned().unwrap_or_default())
    }

    pub fn get_header(&self, name: &RDLTypes) -> RDLTypes {
        RDLTypes::String(self.headers.get(name.to_string().as_str()).cloned().unwrap_or_default())
    }

    pub fn get_body(&self) -> RDLTypes {
        match &self.body {
            HttpBodyVariant::Empty => "".into(),
            HttpBodyVariant::Text(text) => text.clone().into(),
            HttpBodyVariant::Bytes(_) => "[Binary Body - Use body_base64() for content]".into(),
        }
    }

    pub fn get_body_as_base64(&self) -> RDLTypes {
        match &self.body {
            HttpBodyVariant::Text(s) => base64::engine::general_purpose::STANDARD.encode(s.as_bytes()).into(),
            HttpBodyVariant::Bytes(bytes_vec) => base64::engine::general_purpose::STANDARD.encode(bytes_vec).into(),
            HttpBodyVariant::Empty => "".into(),
        }
    }
    
    pub fn is_body_binary(&self) -> bool {
        matches!(&self.body, HttpBodyVariant::Bytes(_))
    }
}