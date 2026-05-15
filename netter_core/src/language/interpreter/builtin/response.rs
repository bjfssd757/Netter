use std::collections::HashMap;
use crate::{language::{interpreter::Object, rdl_types::RDLTypes}, runtime_error};

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub is_sent: bool,
}

impl Object for Response {
    fn name(&self) -> &'static str {
        "Response"
    }

    fn methods(&self) -> Vec<&str> {
        vec!["set_header", "body", "send", "status"]
    }

    fn call_method(&mut self, name: &str, args: Vec<RDLTypes>) -> crate::language::Result<RDLTypes> {
        match name {
            "set_header" => {
                if args.len() < 2 {
                    return runtime_error!("Method Response.set_header required 2 argument");
                }

                self.set_header(&args[0], &args[1]);
                Ok(RDLTypes::Boolean(true))
            }
            "body" => {
                if args.len() < 1 {
                    return runtime_error!("Method Response.body required 1 argument");
                }

                self.body(args[0].to_string());
                Ok(RDLTypes::Boolean(true))
            }
            "send" => {
                self.send();
                Ok(RDLTypes::Boolean(true))
            }
            "status" => {
                if args.len() < 1 {
                    return runtime_error!("Method Response.status required 1 argument");
                }

                let status: u16 = (&args[0]).into();

                if !(100..=599).contains(&status) {
                    return runtime_error!(format!("Incorrect status code: {}", status));
                }

                self.status(status);
                Ok(RDLTypes::Boolean(true))
            },
            _ => runtime_error!(format!("Function with name '{}' not found in Response object", name))
        }
    }

    fn get_property(&self, _name: &str) -> RDLTypes {
        RDLTypes::Boolean(false)
    }

    fn method_exist(&self, name: &str) -> bool {
        self.methods().contains(&name)
    }

    fn properties(&self) -> Vec<&str> {
        vec![]
    }

    fn property_exist(&self, _name: &str) -> bool {
        false
    }
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

    pub fn set_header(&mut self, key: &RDLTypes, value: &RDLTypes) -> &mut Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn is_sent(&self) -> bool {
        self.is_sent
    }
}