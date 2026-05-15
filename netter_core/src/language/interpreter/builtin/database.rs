use log::{trace};
use crate::language::error::{Result};
use crate::language::rdl_types::RDLTypes;
use crate::runtime_error;

pub struct Database {}

impl Database {
    pub fn get_all() -> Result<RDLTypes> {
        // Заглушка
        Ok(r#"[{"id": 1, "name": "User1"}, {"id": 2, "name": "User2"}]"#.into())
    }
    
    pub fn check() -> Result<bool> {
        // Заглушка
        Ok(true)
    }

    pub fn get(user_id: &RDLTypes) -> Result<RDLTypes> {
        if user_id == &RDLTypes::Number(0) {
            runtime_error!(format!("Пользователь с id={} не найден", user_id))
        } else {
            Ok(format!(r#"{{"id": {}, "name": "User{}"}}"#, user_id, user_id).into())
        }
    }

    pub fn add(user_id: &RDLTypes, name: &RDLTypes, password_hash: &RDLTypes) -> Result<()> {
        trace!(
            "Добавлен пользователь: id={}, name={}, password_hash={}",
            user_id, name, password_hash
        );
        Ok(())
    }
}