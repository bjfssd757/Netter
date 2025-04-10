use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use clap::{Parser, Subcommand};
use commands::start::{start_with_config, start_without_config};
use commands::stop::stop;
use crate::state::{load_state, save_state};

mod commands;
mod state;
mod http;

#[allow(async_fn_in_trait)]
pub trait WebSocketTrait {
    fn new(host: String, port: u16, protect: bool) -> Self;
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn default() -> Self;
}

#[derive(Parser, Debug)]
#[command(name = "netter")]
#[command(about = "The Netter will help you create servers easily and quickly.")]
struct Cli {
    #[arg(long)]
    config: Option<String>,

    #[arg(short)]
    verbose: bool,

    #[arg()]
    command: Option<String>,

    #[command(subcommand)]
    subcommand: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start {
        #[arg(long)]
        tcp: bool,

        #[arg(long)]
        udp: bool,

        #[arg(long)]
        websocket: bool,

        #[arg(long)]
        http: bool,

        #[arg(long)]
        path: Option<String>,

        #[arg(long)]
        routes: Option<String>,

        #[arg(long)]
        host: Option<String>,

        #[arg(long)]
        port: Option<u16>,

        #[arg(long, default_value_t = false)]
        protect: bool,
    },
    Stop,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    host: String,
    port: u16,
    protect: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.subcommand {
        Some(Commands::Start { tcp, udp, websocket, http, path, routes, host, port, protect}) => {
            if let Some(path) = path {
                start_with_config(tcp, udp, websocket, http, &path, routes).await?;
            } else {
                start_without_config(tcp, udp, websocket, http, protect, host, port, routes).await?;
            }
            Ok(())
        },
        Some(Commands::Stop) => {
            if let Some(state) = load_state() {
                stop(state.pid).await?;
                Ok(())
            } else {
                Err(Box::<dyn std::error::Error>::from(
                    "Server not running"))
            }
        },
        None => {
            Err(Box::<dyn std::error::Error>::from(
                "No command provided. Use --help for more information."))
        }
    }
}

impl WebSocketTrait for Server {
    fn new(host: String, port: u16, protect: bool) -> Self {
        Self {
            host,
            port,
            protect,
        }
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.protect {
            println!("Server is protected");
            Ok(()) // there should be code for ssl or tls protect
        }
        else {
            println!("Starting server...");

            let addr = format!("{}:{}", self.host, self.port);
            let listener = TcpListener::bind(&addr)
                .await
                .map_err(|e| format!("Failed to bind: {e}"))?;

            save_state(
                String::from("websocket"),
                self.host.clone(),
                self.port.clone()
            )?;

            println!("Server running on {}", &addr);

            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let ws_stream =  match accept_async(stream)
                        .await {
                            Ok(ws) => ws,
                            Err(e) => {
                                eprintln!("Error during WebSocket handshake: {}", e);
                                return
                            }
                        };

                        println!("New connection!");

                    let (mut write, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(msg) => {
                                println!("Received message: {}", msg);
                                if msg.is_text() || msg.is_binary() {
                                    write.send(msg).await.unwrap();
                                }
                            },
                            Err(e) => {
                                eprintln!("Failed while reading message: {e}");
                                return
                            }
                        }
                    }
                });
            }
            Ok(())
        }
    }

    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 8080,
            protect: false,
        }
    }
}
