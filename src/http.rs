use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::state::save_state;

pub trait HttpTrait {
    fn new(host: String, port: u16, protect: bool, cert: Option<String>, key: Option<String>) -> Self;
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn set_route(&self, cfg: String) -> Result<(), Box<dyn std::error::Error>>;
    // fn set_cert(&self, path: String) -> Result<(), Box<dyn std::error::Error>>;
    // fn set_key(&self, path: String) -> Result<(), Box<dyn std::error::Error>>;
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub protect: bool,
    pub cert: Option<String>,
    pub key: Option<String>,
    pub routes: Vec<(String, String, String)>,
    pub response: String,
}

#[derive(Deserialize)]
struct RouteConfig {
    method: String,
    path: String,
    response: String,
}

#[derive(Deserialize)]
struct Config {
    routes: Vec<RouteConfig>,
}

async fn dynamic_handler(response: String) -> impl Responder {
    HttpResponse::Ok().body(response)
}

impl HttpTrait for Server {
    fn new(host: String, port: u16, protect: bool, cert: Option<String>, key: Option<String>) -> Self {
        Server {
            host,
            port,
            protect,
            cert,
            key,
            routes: Vec::new(),
            response: String::new(),
        }
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let routes = self.routes.clone();

        HttpServer::new(move || {
            let mut app = App::new();

            for (method, path, response) in routes.clone() {
                match method.to_lowercase().as_str() {
                    "get" => {
                        app = app.route(&path, web::get().to(move || dynamic_handler(response.clone())));
                    }
                    "post" => {
                        app = app.route(&path, web::post().to(move || dynamic_handler(response.clone())));
                    }
                    "delete" => {
                        app = app.route(&path, web::delete().to(move || dynamic_handler(response.clone())));
                    }
                    _ => {
                        eprintln!("Unsupported method: {}", method);
                    }
                }
            }

            app
        })
        .bind((self.host.as_str(), self.port))?
        .run()
        .await?;

        save_state(String::from("HTTP"), self.host.clone(), self.port.clone())?;

        Ok(())
    }

    fn set_route(&self, cfg: String) -> Result<(), Box<dyn std::error::Error>> {
        let config_data = std::fs::read_to_string(cfg)
            .map_err(|_| "Failed to read config file")?;
        let config: Config = serde_yaml::from_str(&config_data)
            .map_err(|_| "Failed to parse config file")?;

        for route in config.routes {
            println!("Adding route: {} : {}", route.method.clone(), route.path.clone());
            self.routes.clone().push((route.method.clone(), route.path.clone(), route.response.clone()));
        }
        Ok(())
    }

    // fn set_cert(&self, path: String) -> Result<(), Box<dyn std::error::Error>> {
    //     Ok(())
    // }

    // fn set_key(&self, path: String) -> Result<(), Box<dyn std::error::Error>> {
    //     Ok(())
    // }
}