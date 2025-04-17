use crate::core::config_parser::load_config;
use crate::core::servers::webcosket_core::{Server, WebSocketTrait};
use crate::core::servers::http_core;
use crate::core::servers::http_core::HTTP;
use std::process::Command;
use log::{
    info,
    warn,
    error,
    debug,
    trace
};

// pub fn start_parse(path: String) {
//     println!("Go to start parsing");
//     start(path);
// }

// pub fn start_client() {
//     println!("Go to start client");
//     client::start();
// }

pub fn start_client() {
    let output = Command::new("netter")
        .arg("client")
        .output()
        .expect("Failed to start client UI");

    trace!("Client output: {:?}", output);
}

pub async fn start_with_config(tcp: bool, udp: bool, websocket: bool, http: bool, path: &String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Start with params:");
    if tcp {
        info!("TCP: true");
    }
    if udp {
        info!("UDP: true");
    }
    if websocket {
        let conf = load_config(path)?;

        let server: Server = Server::new(
            conf.host.to_lowercase(),
            conf.port,
            conf.protect,
        );
        
        server.start().await
            .map_err(|e| {
                error!("Failed to start websocket server: {}", &e);
                "Failed to start websocket server"
            })?;
    }
    if http {
        let server: http_core::Server = http_core::Server::configure(path)
            .map_err(|e| {
                error!("Failed to configure http server: {}", &e);
                "Failed to configure http server"
            })?;

        server.start().await
            .map_err(|e| {
                error!("Failed to start http server: {}", &e);
                "Failed to start http server"
            })?;
    }
    trace!("Config path: {}", path);

    Ok(())
}

pub async fn start_without_config(tcp: bool, udp: bool, websocket: bool, http: bool, protect: bool, host: Option<String>, port: Option<u16>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Start without config:");

    match &host {
        Some(host) => {
            match &port {
                Some(port) => {
                    if tcp {
                        info!("TCP: true");
                    }
                    if udp {
                        info!("UDP: true");
                    }
                    if websocket {
                        let server: Server = Server::new(host.clone(), port.clone(), protect.clone());

                        server.start().await.map_err(|e| {
                            error!("Failed to start websocket server: {}", &e);
                            "Failed to start websocket server"
                        })?;
                    }
                    if http {
                        warn!("HTTP server not avaible without config!");
                    }
                    info!("Protect: {}", protect);
                    Ok(())
                },
                None => {
                    error!("Port is required when config is not provided!");
                    Err(Box::<dyn std::error::Error>::from(
                        "Port is required when config is not provided!"))
                }
            }
        },
        None => {
            error!("Host is required when config is not provided!");
            Err(Box::<dyn std::error::Error>::from(
                "Host is required when config is not provided!"))
        }
    }
}