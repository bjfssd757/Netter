use language::interpreter;
use serde::{Deserialize, Serialize};
use servers::http_core;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use log::{error, info, warn};

pub mod language;
pub mod servers;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Ping,
    StartServer { config: ConfigSource },
    StopServer { server_id: String },
    GetServerStatus { server_id: String },
    GetAllServersStatus,
    CheckForUpdate,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Response {
    Pong,
    Ok,
    ServerStarted(ServerInfo),
    ServerStopped(String),
    ServerStatus(ServerInfo),
    UpdateAvailable(UpdateInfo),
    UpToDate(String),
    AllServersStatusReport(Vec<ServerInfo>),
    Error(CoreError),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ServerType {
    Http,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConfigSource {
    CustomLangFileContent(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub server_id: String,
    pub server_type: ServerType,
    pub address: String,
    pub pid: Option<u32>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_urls: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CoreError {
    IoError(String),
    SerializationError(String),
    DeserializationError(String),
    InvalidInput(String),
    ConfigParseError(String),
    ServerStartFailed(String),
    ServerStopFailed(String),
    ServerNotFound(String),
    UpdateCheckFailed(String),
    DownloadFailed(String),
    UnsupportedOs(String),
    OperationFailed(String),
    InternalError(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::IoError(s) => write!(f, "IO Error: {}", s),
            CoreError::SerializationError(s) => write!(f, "Serialization Error: {}", s),
            CoreError::DeserializationError(s) => write!(f, "Deserialization Error: {}", s),
            CoreError::InvalidInput(s) => write!(f, "Invalid Input: {}", s),
            CoreError::ConfigParseError(s) => write!(f, "Config Parse Error: {}", s),
            CoreError::ServerStartFailed(s) => write!(f, "Server Start Failed: {}", s),
            CoreError::ServerStopFailed(s) => write!(f, "Server Stop Failed: {}", s),
            CoreError::ServerNotFound(s) => write!(f, "Server Not Found: {}", s),
            CoreError::UpdateCheckFailed(s) => write!(f, "Update Check Failed: {}", s),
            CoreError::DownloadFailed(s) => write!(f, "Download Failed: {}", s),
            CoreError::UnsupportedOs(s) => write!(f, "Unsupported OS: {}", s),
            CoreError::OperationFailed(s) => write!(f, "Operation Failed: {}", s),
            CoreError::InternalError(s) => write!(f, "Internal Error: {}", s),
        }
    }
}
impl StdError for CoreError {}

#[derive(Debug)]
pub enum CoreExecutionResult {
    CliResponse(Response),
    StartHttpServer {
        interpreter: interpreter::Interpreter,
        tls_config: Option<http_core::TlsConfig>,
    },
}

pub async fn execute_core_command(command: Command) -> CoreExecutionResult {
    info!("Core executing command: {:?}", command);
    match command {
        Command::Ping => {
            info!("Core responding with Pong");
            CoreExecutionResult::CliResponse(Response::Pong)
        }
        Command::StartServer { config } => {
            info!("Core processing StartServer command...");
            match config {
                ConfigSource::CustomLangFileContent(content) => {
                    info!("Parsing Custom Language config...");
                    match language::parser::parse(&content) {
                        Ok(ast) => {
                            info!("Parsing successful. Interpreting AST...");
                            let mut interpreter = interpreter::Interpreter::new();
                            match interpreter.interpret(&ast) {
                                Ok(_) => {
                                    info!("Interpretation successful. Preparing HTTP server response.");
                                    let tls_config = interpreter.tls_config.clone();
                                    CoreExecutionResult::StartHttpServer { interpreter, tls_config }
                                }
                                Err(e) => {
                                    error!("Failed to interpret AST: {}", e);
                                    CoreExecutionResult::CliResponse(
                                        Response::Error(
                                            CoreError::ConfigParseError(format!("Interpretation error: {}", e))
                                        )
                                    )
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse custom language config: {}", e);
                            CoreExecutionResult::CliResponse(
                                Response::Error(
                                    CoreError::ConfigParseError(format!("Parsing error: {}", e))
                                )
                            )
                        }
                    }
                }
            }
        }
        Command::StopServer { .. } => {
            info!("Core acknowledged StopServer command. Service will handle it.");
            CoreExecutionResult::CliResponse(Response::Ok)
        }
        Command::GetServerStatus { .. } => {
            info!("Core acknowledged GetServerStatus command. Service will handle it.");
            CoreExecutionResult::CliResponse(Response::Ok)
        }
        Command::GetAllServersStatus => {
            info!("Core acknowledged GetAllServersStatus command. Service will handle it.");
            CoreExecutionResult::CliResponse(
                Response::Ok
            )
        }
        Command::CheckForUpdate => {
            info!("Core processing CheckForUpdate command...");
            warn!("Update check functionality is not implemented.");
            CoreExecutionResult::CliResponse(
                Response::UpToDate(env!("CARGO_PKG_VERSION").to_string())
            )
        }
    }
}