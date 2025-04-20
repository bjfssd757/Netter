use clap::{Parser, Subcommand};
use commands::start::{self, start_with_config, start_without_config};
use commands::stop::stop;
use commands::update::update;
use crate::state::load_state;
use crate::commands::macros as logger;
use log::{
    info,
    error,
    trace
};

mod commands;
mod state;
mod core;

#[derive(Parser, Debug)]
#[command(name = "netter")]
#[command(about = "The Netter will help you create servers easily and quickly.")]
#[command(version = env!("CARGO_PKG_VERSION"), author = "bjfssd757")]
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
        #[arg(long = "tcp", short = 't')]
        tcp: bool,

        #[arg(long = "udp", short = 'u')]
        udp: bool,

        #[arg(long = "websocket", short = 'w')]
        websocket: bool,

        #[arg(long = "http", short = 'h')]
        http: bool,

        #[arg(long = "path")]
        path: Option<String>,

        #[arg(long = "udp")]
        host: Option<String>,

        #[arg(long = "port", short = 'p')]
        port: Option<u16>,

        #[arg(long, default_value_t = false)]
        protect: bool,
    },
    Stop,
    Parse {
        #[arg(long)]
        path: Option<String>,

        #[arg(short)]
        p: Option<String>,
    },
    Client,
    Update,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // let name: String = logger::generate_name();

    match cli.subcommand {
        Some(Commands::Start { tcp, udp, websocket, http, path, host, port, protect}) => {
            if let Err(e) = logger::init(None) {
                error!("Failed to initialize logger: {}", e);
                Err(Box::<dyn std::error::Error>::from(
                    "Failed to initialize logger"
                ))?;
            }
        
            trace!("Logger initialized successfully");
            
            if let Some(path) = path {
                info!("Starting with conifg: {}", path.clone());
                start_with_config(tcp, udp, websocket, http, &path).await?;
            } else {
                info!("Starting without config");
                start_without_config(tcp, udp, websocket, http, protect, host, port).await?;
            }
            Ok(())
        },
        Some(Commands::Stop) => {
            if let Err(e) = logger::init(None) {
                error!("Failed to initialize logger: {}", e);
                Err(Box::<dyn std::error::Error>::from(
                    "Failed to initialize logger"
                ))?;
            }
        
            trace!("Logger initialized successfully");

            if let Some(state) = load_state() {
                stop(state.pid).await?;
                Ok(())
            } else {
                Err(Box::<dyn std::error::Error>::from(
                    "Server not running"))
            }
        },
        Some(Commands::Parse { path, p }) => {
            if let Err(e) = logger::init(None) {
                error!("Failed to initialize logger: {}", e);
                Err(Box::<dyn std::error::Error>::from(
                    "Failed to initialize logger"
                ))?;
            }
        
            trace!("Logger initialized successfully");

            if let Some(path) = path {
                commands::start::start_parse(path).await
                .map_err(|e| {
                    error!("Failed to parse file: {}", &e);
                    "failed to parse file"
                })?;
            }
            if let Some(p) = p {
                if let Err(e) = logger::init(None) {
                    error!("Failed to initialize logger: {}", e);
                    Err(Box::<dyn std::error::Error>::from(
                        "Failed to initialize logger"
                    ))?;
                }
            
                trace!("Logger initialized successfully");

                commands::start::start_parse(p).await
                .map_err(|e| {
                    error!("Failed to parse file: {}", &e);
                    "failed to parse file"
                })?;
            }
            Ok(())
        },
        Some(Commands::Client) => {
            if let Err(e) = logger::init(None) {
                error!("Failed to initialize logger: {}", e);
                Err(Box::<dyn std::error::Error>::from(
                    "Failed to initialize logger"
                ))?;
            }
        
            trace!("Logger initialized successfully");

            start::start_client()
                .map_err(|e| {
                    error!("Failed to start client: {}", &e);
                    "failed to start client"
                })?;
            Ok(())
        },
        Some(Commands::Update) => {
            if let Err(e) = logger::init(None) {
                error!("Failed to initialize logger: {}", e);
                Err(Box::<dyn std::error::Error>::from(
                    "Failed to initialize logger"
                ))?;
            }
        
            trace!("Logger initialized successfully");

            update()
                .map_err(|e| {
                    error!("Failed to update: {}", &e);
                    "failed to update"
                })?;
            Ok(())
        }
        None => {
            Err(Box::<dyn std::error::Error>::from(
                "No command provided. Use --help for more information."))
        }
    }
}
