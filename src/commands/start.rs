use crate::core::config_parser::load_config;
use crate::core::language::interpreter::Interpreter;
use crate::core::servers::webcosket_core::{Server, WebSocketTrait};
use crate::core::servers::http_core;
use crate::core::servers::http_core::HTTP;
use crate::core::language::parser;
use std::process::Command;
use std::process::ExitStatus;
use log::{
    info,
    warn,
    error,
    debug,
    trace
};

pub async fn start_parse(path: String) -> Result<(), Box<dyn std::error::Error>> {
    trace!("Go to start parsing");
    let code = std::fs::read_to_string(path)
        .map_err(|e| error!("Failed to read file: {e}")).unwrap();
    let ast = parser::parse(&code)
        .map_err(|e| error!("Failed to parse file: {e}")).unwrap();
    let mut interpreter = Interpreter::new();
    if let Err(e) = interpreter.interpret(&ast) {
        error!("Failed to interpret AST: {}", e);
    }

    trace!("File parsed successfully!");

    let addr = vec![127, 0, 0, 1];
    let port = 9090;

    info!("Creating server on {}:{} with TLS: {}", 
        addr.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("."),
        port,
        if interpreter.tls_config.is_some() { "enabled" } else { "disabled" }
    );

    let server = if let Some(tls_config) = interpreter.tls_config.clone() {
        info!("Starting HTTPS server with TLS enabled");
        if !tls_config.enabled {
            info!("TLS configuration found but disabled, using HTTP");
        }
        http_core::Server::from_interpreter(addr, port, interpreter, Some(tls_config))
    } else {
        info!("No TLS configuration found, using HTTP");
        http_core::Server::from_interpreter(addr, port, interpreter, None)
    };

    info!("Starting server...");

    server.start().await
        .map_err(|e| error!("Failed to start server: {e}")).unwrap();

    Ok(())
}

pub fn start_client() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting client...");

    if cfg!(target_os = "windows") {
        Command::new("./setup_dependencies.bat")
            .spawn()
            .map_err(|e| {
                error!("Failed to execute setup_dependencies.py: {e}");
                "Failed to execute setup_dependencies.py"
            })?;
    } else {
        Command::new("python3")
            .arg("setup_dependencies.py")
            .spawn()
            .map_err(|e| {
                error!("Failed to execute setup_dependencies.py: {e}");
                "Failed to execute setup_dependencies.py"
            })?;
    }
    Ok(())
}

pub async fn start_with_config(tcp: bool, udp: bool, websocket: bool, http: bool, path: &String) -> Result<(), Box<dyn std::error::Error>> {
    trace!("Start with params:");
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
    trace!("Start without config:");

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