use std::fs;
use crate::{Server, ServerTrait};

pub async fn start_with_config(tcp: bool, udp: bool, websocket: bool, path: &String) {
    println!("Start with params:");
    if tcp {
        println!("TCP: true");
    }
    if udp {
        println!("UDP: true");
    }
    if websocket {
        let config = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read config file"));

        let server: Server = toml::from_str(&config)
            .unwrap_or_else(|_| panic!("Failed to parse config file"));
        
        server.start().await;
    }
    println!("Config path: {}", path);
}

pub async fn start_without_config(tcp: bool, udp: bool, websocket: bool, protect: bool, host: Option<String>, port: Option<u16>) {
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

                        server.start().await;
                    }
                    println!("Protect: {}", protect);
                },
                None => {
                    panic!("Port is required when config is not provided!");
                }
            }
        },
        None => {
            println!("Host is required when config is not provided!");
        }
    }
}