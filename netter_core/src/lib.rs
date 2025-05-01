use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    Ping,
    StartServer {
        kind: String,
        host: String,
        port: u16,
        protect: bool,
    },
    CreateServer {
        kind: String,
        host: String,
        port: u16,
        protect: bool,
    },
    StopServer {
        server_id: u32,
    },
    ShutdownServer { // extremal stopping server
        server_id: u32,
    },
    GetStatus {
        server_id: Option<u32>,
    },
    ServerState {
        id: u32,
    },
    GetAllServers,
    Auth {
        user: String,
    },
    Update,
    Parse {
        path: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SuccessData {
    Pong,
    ServerStarted { task_id: u32 },
    ServerStopped,
    ShutdownServer,
    ServerCreated { task_id: u32 },
    StatusReport { status: String },
    StateReport {
        is_running: bool,
        id: u32,
        kind: String,
        host: String,
        port: u16,
        protect: bool,
        uptime: u64, // in seconds
        connections: u32,
        logs: Vec<String>,
    },
    AllServersReport {
        servers: Vec<ServerInfo>,
    },
    AuthSuccess,
    UpdateSuccess,
    ParseSuccess,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerInfo {
    pub id: u32,
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub protect: bool,
    pub uptime: u64, // in seconds
    pub connections: u32,
    pub logs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Response {
    Success(SuccessData),
    Error(CoreError),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoreError {
    IoError(String),
    TaskNotFound(u32),
    InvalidInput(String),
    InternalError(String),
    ServerNotFound(u32),
    ServerAlreadyRunning(u32),
    ServerAlreadyStopped(u32),
    ServerNotRunning(u32),
    InvalidUser(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::IoError(s) => write!(f, "IO Error: {}", s),
            CoreError::TaskNotFound(id) => write!(f, "Task not found: {}", id),
            CoreError::InvalidInput(s) => write!(f, "Invalid input: {}", s),
            CoreError::InternalError(s) => write!(f, "Internal error: {}", s),
            CoreError::ServerAlreadyRunning(s) => write!(f, "Server already running: {}", s),
            CoreError::ServerAlreadyStopped(s) => write!(f, "Server already stopped: {}", s),
            CoreError::ServerNotRunning(s) => write!(f, "Server not running: {}", s),
            CoreError::ServerNotFound(s) => write!(f, "Server not found: {}", s),
            CoreError::InvalidUser(s) => write!(f, "Invalid user: {}", s),
        }
    }
}

impl std::error::Error for CoreError {}

pub async fn execute_command(command: Command) -> Response {
    log::info!("Executing command: {:?}", &command);

    let result: Result<SuccessData, CoreError> = match command {
        Command::Ping => handle_ping().await,
        Command::CreateServer { kind, host, port, protect } => {
            create_server(kind, port, host, protect).await
        },
        Command::StartServer { kind, host, port, protect } => {
            start_server(kind, host, port, protect).await
        },
        Command::StopServer { server_id } => stop_server(server_id).await,
        Command::ShutdownServer { server_id } => shutdown_server(server_id).await,
        Command::GetStatus { server_id } => get_status(server_id).await,
        Command::ServerState { id } => server_state(id).await,
        Command::GetAllServers => get_all_servers().await,
        Command::Auth { user } => auth_user(user).await,
        Command::Update => {
            log::info!("Update command received");
            Ok(SuccessData::UpdateSuccess)
        },
        Command::Parse { path } => {
            log::info!("Parse command received for path: {}", path);
            Ok(SuccessData::ParseSuccess)
        },
    };

    match result {
        Ok(success_data) => Response::Success(success_data),
        Err(error) => {
            log::error!("Error executing command: {}", error);
            Response::Error(error)
        }
    }
}

async fn handle_ping() -> Result<SuccessData, CoreError> {
    log::info!("Ping command received");
    Ok(SuccessData::Pong)
}

async fn create_server(kind: String, port: u16, host: String, protect: bool) -> Result<SuccessData, CoreError> {
    log::info!("Creating server with kind: {}, host: {}, port: {}, protect: {}", kind, host, port, protect);
    Ok(SuccessData::ServerCreated { task_id: 1 }) // Заглушка
}

async fn start_server(kind: String, host: String, port: u16, protect: bool) -> Result<SuccessData, CoreError> {
    log::info!("Starting server with kind: {}, host: {}, port: {}, protect: {}", kind, host, port, protect);
    Ok(SuccessData::ServerStarted { task_id: 1 }) // Заглушка
}

async fn stop_server(server_id: u32) -> Result<SuccessData, CoreError> {
    log::info!("Stopping server with ID: {}", server_id);
    Ok(SuccessData::ServerStopped)
}

async fn shutdown_server(server_id: u32) -> Result<SuccessData, CoreError> {
    log::info!("Shutting down server with ID: {}", server_id);
    Ok(SuccessData::ShutdownServer)
}

async fn get_status(server_id: Option<u32>) -> Result<SuccessData, CoreError> {
    log::info!("Getting status for server ID: {:?}", server_id);
    Ok(SuccessData::StatusReport { status: "OK".to_string() })
}

async fn server_state(id: u32) -> Result<SuccessData, CoreError> {
    log::info!("Getting state for server ID: {}", id);
    Ok(SuccessData::StateReport {
        is_running: true,
        id,
        kind: "HTTP".to_string(),
        host: "localhost".to_string(),
        port: 8080,
        protect: false,
        uptime: 3600,
        connections: 100,
        logs: vec!["Log entry 1".to_string(), "Log entry 2".to_string()],
    })
}

async fn get_all_servers() -> Result<SuccessData, CoreError> {
    log::info!("Getting all servers");
    Ok(SuccessData::AllServersReport {
        servers: vec![
            ServerInfo {
                id: 1,
                kind: "HTTP".to_string(),
                host: "localhost".to_string(),
                port: 8080,
                protect: false,
                uptime: 3600,
                connections: 100,
                logs: vec!["Log entry 1".to_string(), "Log entry 2".to_string()],
            },
            ServerInfo {
                id: 2,
                kind: "TCP".to_string(),
                host: "localhost".to_string(),
                port: 9090,
                protect: true,
                uptime: 7200,
                connections: 200,
                logs: vec!["Log entry A".to_string(), "Log entry B".to_string()],
            },
        ],
    })
}

async fn auth_user(user: String) -> Result<SuccessData, CoreError> {
    log::info!("Authenticating user: {}", user);
    if user == "admin" {
        Ok(SuccessData::AuthSuccess)
    } else {
        Err(CoreError::InvalidUser(user))
    }
}