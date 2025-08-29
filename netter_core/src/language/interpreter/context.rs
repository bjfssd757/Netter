use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    variables: HashMap<String, String>,
    parent: Option<Box<ExecutionContext>>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        ExecutionContext {
            variables: HashMap::new(),
            parent: None,
        }
    }
    
    pub fn with_parent(parent: ExecutionContext) -> Self {
        ExecutionContext {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn set_variable(&mut self, name: &str, value: String) {
        self.variables.insert(name.to_string(), value);
    }

    pub fn get_variable(&self, name: &str) -> Option<String> {
        if let Some(value) = self.variables.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_variable(name)
        } else {
            None
        }
    }

    pub fn has_variable_local(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    pub fn has_variable(&self, name: &str) -> bool {
        if self.has_variable_local(name) {
            true
        } else if let Some(parent) = &self.parent {
            parent.has_variable(name)
        } else {
            false
        }
    }

    pub fn get_local_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }
}