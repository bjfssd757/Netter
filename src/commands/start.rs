use crate::core::config_parser::load_config;
use crate::core::servers::webcosket_core::{Server, WebSocketTrait};
use crate::core::servers::http_core;
use crate::core::servers::http_core::HTTP;

// pub fn start_parse(path: String) {
//     println!("Go to start parsing");
//     start(path);
// }

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
        let conf = load_config(path)?;

        let server: Server = Server::new(
            conf.host.to_lowercase(),
            conf.port,
            conf.protect,
        );
        
        server.start().await?;
    }
    if http {
        let server: http_core::Server = http_core::Server::configure(path)
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