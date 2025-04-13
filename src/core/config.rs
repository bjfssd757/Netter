use std::io::Write;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use serde::Deserialize;


#[derive(Debug)]
pub enum Commands {
    Kind(String),
    Host(String),
    Port(u16),
    Routes(RouteConfig),
    RoutesDefinitionLanguagePath(String),
    Timeout(i64),
    TimeoutConnect(i64),
    Logging(Logger),
    Ssl(SslConfigure),
    Pool(PoolConfigure),
    Enviroment(EnviromentConfigure),
    Cache(CacheConfigure),
    Monitoring(MonitoringConfigure),
}

#[derive(Debug, Deserialize)]
pub struct Response {
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
    connect: Option<Database>,
    disconnect: Option<Database>,
}

#[derive(Debug, Deserialize)]
pub struct MonitoringConfigure {
    pub enable: bool,
    pub endpoint: String,
}

#[derive(Debug, Deserialize)]
pub struct CacheConfigure {
    pub enable: bool,
    pub kind: String,
    pub ttl: i64,
    pub host: String,
    pub port: i64,
}

#[derive(Debug, Deserialize)]
pub struct EnviromentConfigure {
    pub database_url: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct PoolConfigure {
    pub max_connection: i64,
    pub min_connection: i64,
}

#[derive(Debug, Deserialize)]
pub struct SslConfigure {
    pub enable: bool,
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Deserialize)]
pub struct Logger {
    pub level: String,
    pub file: String,
}

#[derive(Debug, Deserialize)]
pub struct RouteConfig {
    pub path: String,
    pub method: String,
    pub response: Response,
}

#[derive(Debug, Deserialize)]
struct Database {
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