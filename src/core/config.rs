use std::io::Write;
use diesel::prelude::*;
use diesel::pg::PgConnection;

use super::models;

#[derive(Debug)]
pub enum Commands {
    Routes(RouteConfig),
}

#[derive(Debug)]
pub enum Response {
    Status(i64),
    Body(String),
    Headers(Vec<(String, String)>),
    Connect(Database),
    Disconnect(Database),
}

#[derive(Debug)]
pub struct RouteConfig {
    path: String,
    method: String,
    response: Response,
}

#[derive(Debug)]
pub struct Database {
    uri: String,
    user_name: String,
    password: String,
}

pub trait DatabaseTrait {
    type Error;
    fn init(uri: String, user_name: String, password: String) -> Self;
    fn set_connection(&self) -> Result<PgConnection, Self::Error>;
    //fn get_all(connection: &PgConnection) -> Vec<models::User>;
}


impl DatabaseTrait for Database {
    type Error = Box<dyn std::error::Error>;

    fn init(uri: String, user_name: String, password: String) -> Self {
        let mut enviroment = match std::fs::File::create(".env") {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Error creating .env file: {}", e);
                panic!("Failed to create .env file");
            },
        };

        enviroment.write(format!("DATABASE_URL={}", uri.clone()).as_bytes());

        Self {
            uri,
            user_name,
            password,
        }
    }

    fn set_connection(&self) -> Result<PgConnection, Self::Error> {
        dotenvy::dotenv().ok();

        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let connect = PgConnection::establish(&url)?;
        
        Ok(connect)
    }
}