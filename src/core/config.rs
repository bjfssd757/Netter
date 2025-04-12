use std::io::Write;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use serde::Deserialize;


#[derive(Debug)]
pub enum Commands {
    Routes(RouteConfig),
    RoutesDefinitionLanguagePath(String),
    Timeout(i64),
    TimeoutConnect(i64),
    Logging(Logger),
    Ssl(SslConfigure),
    Pool(PoolConfigure),
    Enviroment(EnviromentConfigure),
    Cache(CacheConfigure),
    Monitoring(),
}

#[derive(Debug, Deserialize)]
pub enum Response {
    Status(i64),
    Body(String),
    Headers(Vec<(String, String)>),
    Connect(Database),
    Disconnect(Database),
}

#[derive(Debug, Deserialize)]
pub struct MonitoringConfigure {
    enable: bool,
    endpoint: String,
}

#[derive(Debug, Deserialize)]
pub struct CacheConfigure {
    enable: bool,
    kind: String,
    ttl: i64,
    host: String,
    port: i64,
}

#[derive(Debug, Deserialize)]
pub struct EnviromentConfigure {
    database_url: String,
    api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct PoolConfigure {
    max_connection: i64,
    min_connection: i64,
}

#[derive(Debug, Deserialize)]
pub struct SslConfigure {
    enable: bool,
    cert_path: String,
    key_path: String,
}

#[derive(Debug, Deserialize)]
pub struct Logger {
    level: String,
    file: String,
}

#[derive(Debug, Deserialize)]
pub struct RouteConfig {
    path: String,
    method: String,
    response: Response,
}

#[derive(Debug, Deserialize)]
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

        enviroment.write(format!("DATABASE_URL={}", uri.clone()).as_bytes())
            .map_err(|e| {
                eprintln!("Error writing to .env file: {}", e);
                panic!("Failed to write to .env file");
            }).unwrap();

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