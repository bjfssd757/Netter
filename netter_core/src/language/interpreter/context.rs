#![allow(dead_code)]

use std::collections::HashMap;
use netter_sdk::{Object, RDLTypes};

pub struct ExecutionContext {
    variables: HashMap<RDLTypes, RDLTypes>,
    objects: Vec<Box<dyn Object>>,
    parent: Option<Box<ExecutionContext>>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        ExecutionContext {
            variables: HashMap::new(),
            objects: Vec::new(),
            parent: None,
        }
    }
    
    pub fn with_parent(parent: ExecutionContext) -> Self {
        ExecutionContext {
            variables: HashMap::new(),
            objects: Vec::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn add_object(&mut self, object: Box<impl Object>) {
        self.objects.push(object);
    }

    pub fn get_object(&mut self, name: &RDLTypes) -> Option<&dyn Object> {
        self.objects.iter()
            .find(|o| o.name() == name.to_string())
            .map(|obj| obj.as_ref())
    }

    pub fn has_object(&self, name: &RDLTypes) -> bool {
        self.objects.iter()
            .find(|o| o.name() == name.to_string())
            .is_some()
    }

    pub fn get_objects(&self) -> Vec<&dyn Object> {
        self.objects.iter().map(|o| o.as_ref()).collect()
    }

    pub fn set_variable(&mut self, name: &RDLTypes, value: RDLTypes) {
        self.variables.insert(name.clone().into(), value);
    }

    pub fn get_variable(&self, name: &RDLTypes) -> Option<RDLTypes> {
        if let Some(value) = self.variables.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_variable(name)
        } else {
            None
        }
    }

    pub fn has_variable_local(&self, name: &RDLTypes) -> bool {
        self.variables.contains_key(name)
    }

    pub fn has_variable(&self, name: &RDLTypes) -> bool {
        if self.has_variable_local(name) {
            true
        } else if let Some(parent) = &self.parent {
            parent.has_variable(name)
        } else {
            false
        }
    }

    pub fn get_local_variables(&self) -> &HashMap<RDLTypes, RDLTypes> {
        &self.variables
    }
}