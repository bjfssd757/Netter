use crate::core::config_parser::load_config;
use crate::core::servers::webcosket_core::{Server, WebSocketTrait};
use crate::core::servers::http_core;
use crate::core::servers::http_core::HTTP;
use crate::core::language::parser;
use std::process::Command;
use log::{
    info,
    warn,
    error,
    debug,
    trace
};

pub async fn start_parse(path: String) {
    trace!("Go to start parsing");
    let code = std::fs::read_to_string(path)
        .map_err(|e| error!("Failed to read file: {e}")).unwrap();
    let ast = parser::parse(&code)
        .map_err(|e| error!("Failed to parse file: {e}")).unwrap();

    trace!("File parsed successfully!");

    let addr = vec![127, 0, 0, 1];
    let port = 9090;

    let server = http_core::Server::from_ast(addr, port, &ast)
        .map_err(|e| error!("Failed to create server: {e}")).unwrap();

    server.start().await
        .map_err(|e| error!("Failed to start server: {e}")).unwrap();
}

// pub fn start_client() {
//     println!("Go to start client");
//     client::start();
// }

pub fn start_client() {
    debug!("Creating build directory...");

    Command::new("cmake")
        .arg("-S")
        .arg(".")
        .arg("-B")
        .arg("build")
        .arg("-G")
        .arg("Ninja")
        .output()
        .expect("Failed to create build directory client UI");

    debug!("Building...");

    Command::new("cmake")
        .arg("--build")
        .arg("build")
        .output()
        .expect("Failed to build client UI");

    debug!("Starting client UI...");

    Command::new("build/bin/NetterUI.exe").output()
        .expect("Failed to start client UI");
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