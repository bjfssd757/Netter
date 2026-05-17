use log::{trace};
use crate::language::error::{Result as CoreResult};
use crate::runtime_error;
use netter_sdk::{RDLTypes, Object};

pub struct Database {}

impl Object for Database {
    fn name(&self) -> &'static str {
        "Database"
    }

    fn methods(&self) -> Vec<&str> {
        vec!["get_all", "check", "get", "add"]
    }

    fn properties(&self) -> Vec<&str> {
        Vec::new()
    }

    fn call_method(&mut self, name: &str, args: Vec<RDLTypes>) -> Result<RDLTypes, String> {
        match name {
            "get_all" => Ok(Database::get_all()?),
            "check" => Ok(Database::check().unwrap_or(false).into()),
            "get" => Ok(Database::get(&args[0])?),
            "add" => {
                Database::add(&args[0], &args[1], &args[2])?;
                return Ok(RDLTypes::Boolean(true));
            }
            _ => Err(format!("Function with name '{}' not found in Database object", name))
        }
    }

    fn get_property(&self, _name: &str) -> RDLTypes {
        RDLTypes::Boolean(false)
    }

    fn method_exist(&self, name: &str) -> bool {
        self.methods().contains(&name)
    }

    fn property_exist(&self, _name: &str) -> bool {
        false
    }
}

impl Database {
    pub fn get_all() -> CoreResult<RDLTypes> {
        // Заглушка
        Ok(r#"[{"id": 1, "name": "User1"}, {"id": 2, "name": "User2"}]"#.into())
    }
    
    pub fn check() -> CoreResult<bool> {
        // Заглушка
        Ok(true)
    }

    pub fn get(user_id: &RDLTypes) -> CoreResult<RDLTypes> {
        if user_id == &RDLTypes::Number(0) {
            runtime_error!(format!("Пользователь с id={} не найден", user_id))
        } else {
            Ok(format!(r#"{{"id": {}, "name": "User{}"}}"#, user_id, user_id).into())
        }
    }

    pub fn add(user_id: &RDLTypes, name: &RDLTypes, password_hash: &RDLTypes) -> CoreResult<()> {
        trace!(
            "Добавлен пользователь: id={}, name={}, password_hash={}",
            user_id, name, password_hash
        );
        Ok(())
    }
}