use std::fs;
use crate::{Server, WebSocketTrait};
use crate::core::http_core;
use crate::core::http_core::HTTP;
use crate::core::language::parser::start;

pub fn start_parse(path: String) {
    println!("Go to start parsing");
    start(path);
}

// pub fn start_client() {
//     println!("Go to start client");
//     client::start();
// }

pub async fn start_with_config(tcp: bool, udp: bool, websocket: bool, http: bool, path: &String) -> Result<(), Box<dyn std::error::Error>> {
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
        let server: http_core::Server = http_core::Server::from_config_file(path)
            .map_err(|_| "Failed to parse config file")?;

        server.start().await
            .map_err(|_| "Failed to start server")?;
    }
    println!("Config path: {}", path);

    Ok(())
}

pub async fn start_without_config(tcp: bool, udp: bool, websocket: bool, http: bool, protect: bool, host: Option<String>, port: Option<u16>) -> Result<(), Box<dyn std::error::Error>> {
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
                        eprintln!("HTTP server not avaible without config!");
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