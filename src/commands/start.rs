use std::fs;
use crate::{Server, WebSocketTrait};
use crate::http;
use crate::http::HttpTrait;

pub async fn start_with_config(tcp: bool, udp: bool, websocket: bool, http: bool, path: &String, routes: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Start with params:");
    if tcp {
        println!("TCP: true");
    }
    if udp {
        println!("UDP: true");
    }
    if websocket {
        let config = fs::read_to_string(path)
            .map_err(|_| "Failed to read config file")?;

        let server: Server = toml::from_str(&config)
            .map_err(|_|"Failed to parse config file")?;
        
        server.start().await?;
    }
    if http {
        let config = fs::read_to_string(path)
            .map_err(|_| "Failed to read config file")?;

        let server: http::Server = toml::from_str(&config)
            .map_err(|_|"Failed to parse config file")?;

        if let Some(routes) = routes.clone() {
            server.set_route(routes)?;
            server.start().await?;
        } else {
            return Err(Box::<dyn std::error::Error>::from(
                "Routes are required when server type is HTTP provided!"))
        }
    }
    println!("Config path: {}", path);

    Ok(())
}

pub async fn start_without_config(tcp: bool, udp: bool, websocket: bool, http: bool, protect: bool, host: Option<String>, port: Option<u16>, routes: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Start without config:");

    match &host {
        Some(host) => {
            match &port {
                Some(port) => {
                    if tcp {
                        println!("TCP: true");
                    }
                    if udp {
                        println!("UDP: true");
                    }
                    if websocket {
                        let server: Server = Server::new(host.clone(), port.clone(), protect.clone());

                        server.start().await?;
                    }
                    if http {
                        let server: http::Server = http::Server::new(host.clone(), port.clone(), protect.clone(), None, None);

                        if let Some(routes) = routes.clone() {
                            server.set_route(routes)?;

                            server.start().await?;
                        } else {
                            return Err(Box::<dyn std::error::Error>::from(
                                "Routes are required when server type is HTTP provided!"))
                        }
                    }
                    println!("Protect: {}", protect);
                    Ok(())
                },
                None => {
                    Err(Box::<dyn std::error::Error>::from(
                        "Port is required when config is not provided!"))
                }
            }
        },
        None => {
            Err(Box::<dyn std::error::Error>::from(
                "Host is required when config is not provided!"))
        }
    }
}