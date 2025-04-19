use clap::{Parser, Subcommand};
use commands::start::{self, start_with_config, start_without_config};
use commands::stop::stop;
use crate::state::load_state;
use crate::commands::macros as logger;
use log::{
    info,
    warn,
    error,
    debug,
    trace
};

mod commands;
mod state;
mod core;

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
        host: Option<String>,

        #[arg(long)]
        port: Option<u16>,

        #[arg(long, default_value_t = false)]
        protect: bool,
    },
    Stop,
    Parse {
        #[arg(long)]
        path: String,
    },
    Client,
    Version,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name: String = logger::generate_name();

    if let Err(e) = logger::init(Some(&name)) {
        eprintln!("Failed to initialize logger: {}", e);
        return Err(Box::<dyn std::error::Error>::from(
            "Failed to initialize logger"));
    }

    trace!("Logger initialized successfully");

    let cli = Cli::parse();

    match cli.subcommand {
        Some(Commands::Start { tcp, udp, websocket, http, path, host, port, protect}) => {
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
            if let Some(state) = load_state() {
                stop(state.pid).await?;
                Ok(())
            } else {
                Err(Box::<dyn std::error::Error>::from(
                    "Server not running"))
            }
        },
        Some(Commands::Parse { path }) => {
            commands::start::start_parse(path).await;
            Ok(())
        },
        Some(Commands::Client) => {
            start::start_client();
            Ok(())
        },
        Some(Commands::Version) => {
            println!("Netter version 0.3.0");
            Ok(())
        }
        None => {
            Err(Box::<dyn std::error::Error>::from(
                "No command provided. Use --help for more information."))
        }
    }
}
