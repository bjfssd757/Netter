use log::{trace};
use crate::language::error::{Result};
use crate::runtime_error;

pub struct Database {}

impl Database {
    pub fn get_all() -> Result<String> {
        // Заглушка
        Ok(r#"[{"id": 1, "name": "User1"}, {"id": 2, "name": "User2"}]"#.to_string())
    }
    
    pub fn check() -> Result<bool> {
        // Заглушка
        Ok(true)
    }

    pub fn get(user_id: &str) -> Result<String> {
        if user_id == "0" {
            runtime_error!(format!("Пользователь с id={} не найден", user_id))
        } else {
            Ok(format!(r#"{{"id": {}, "name": "User{}"}}"#, user_id, user_id))
        }
    }

    pub fn add(user_id: &str, name: &str, password_hash: &str) -> Result<()> {
        trace!(
            "Добавлен пользователь: id={}, name={}, password_hash={}",
            user_id, name, password_hash
        );
        Ok(())
    }
}