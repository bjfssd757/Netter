use std::collections::HashMap;
use base64::Engine;
use netter_sdk::{RDLTypes, Object};

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

impl Object for Request {
    fn name(&self) -> &'static str {
        "Request"
    }

    fn methods(&self) -> Vec<&str> {
        vec![
            "get_param", "get_header", 
            "body", "text_body", "body_base64", "is_binary"
        ]
    }

    fn call_method(&mut self, name: &str, args: Vec<RDLTypes>) -> Result<RDLTypes, String> {
        match name {
            "get_param" => {
                if args.len() < 1 {
                    return Err(format!("Method Request.get_param required 1 argument"));
                }

                Ok(self.get_param(&args[0]))
            }
            "get_header" => {
                if args.len() < 1 {
                    return Err(format!("Method Request.get_header required 1 argument"));
                }

                Ok(self.get_header(&args[0]))
            }
            "body" | "text_body" => Ok(self.get_body()),
            "body_base64" => Ok(self.get_body_as_base64()),
            "is_binary" => Ok(self.is_body_binary().into()),
            _ => Err(format!("Function with name '{}' not found in Request object", name))
        }
    }

    fn get_property(&self, _name: &str) -> RDLTypes {
        RDLTypes::Boolean(false)
    }

    fn method_exist(&self, name: &str) -> bool {
        self.methods().contains(&name)
    }

    fn properties(&self) -> Vec<&str> {
        Vec::new()
    }

    fn property_exist(&self, _name: &str) -> bool {
        false
    }
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