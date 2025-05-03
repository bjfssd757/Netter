use std::{
    collections::HashMap,
    error::Error as StdError,
    fs,
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn, LevelFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc as tokio_mpsc,
    task::JoinHandle,
};
use netter_core::{
    servers::http_core::{self, Server as HttpCoreServer},
    Command,
    CoreError,
    CoreExecutionResult,
    Response,
    ServerInfo,
    ServerType,
};
use netter_logger;

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::sync::mpsc;
#[cfg(windows)]
use tokio::net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions};
#[cfg(windows)]
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, Result as WindowsServiceResult,
};

#[cfg(unix)]
use directories_next::ProjectDirs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

#[allow(dead_code)]
const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "Netter";
const APPLICATION: &str = "NetterService";

#[derive(Debug, Serialize, Deserialize)]
struct RunningServer {
    info: ServerInfo,
    #[serde(skip)]
    task_handle: Option<JoinHandle<()>>,
}

lazy_static! {
    static ref STATE_FILE_PATH: PathBuf = {
        #[cfg(windows)] {
            Path::new("C:\\ProgramData")
                .join(ORGANIZATION)
                .join(APPLICATION)
                .join("state.bin")
            }
        #[cfg(unix)] {
            ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
                .map(|d| d.data_local_dir()
                    .join("state.bin"))
                    .unwrap_or_else(|| Path::new("/var/lib")
                        .join(APPLICATION
                            .to_lowercase())
                            .join("state.bin"))
        }
        #[cfg(not(any(windows, unix)))] {
            PathBuf::from("netter_state.bin")
        }
    };
    static ref LOG_PATH: PathBuf = {
         #[cfg(windows)] {
            PathBuf::from("C:\\ProgramData")
                .join(ORGANIZATION)
                .join(APPLICATION)
                .join("Logs")
        }
         #[cfg(unix)] {
            ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
                .map(|d| d.data_local_dir().join("netterd.log"))
                .unwrap_or_else(|| Path::new("/var/log")
                    .join(APPLICATION
                        .to_lowercase())
                        .join("netterd.log"))
        }
        #[cfg(not(any(windows, unix)))] {
            PathBuf::from("netter_service.log")
        }
    };
    static ref RUNNING_SERVERS: Arc<Mutex<HashMap<String, RunningServer>>> = Arc::new(Mutex::new(HashMap::new()));
}

fn get_state_file_path_with_create_dir() -> Option<PathBuf> {
    let path = &*STATE_FILE_PATH;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("Failed create state dir {}: {}", parent.display(), e);
            }
            #[cfg(unix)]
            {
                match fs::metadata(parent) {
                    Ok(m) => {
                        let mut p = m.permissions();
                        p.set_mode(0o770);
                        let _ = fs::set_permissions(parent, p);
                    }
                    Err(e) => warn!("Failed get/set state dir perms {}: {}", parent.display(), e),
                }
            }
        }
    }
    Some(path.to_path_buf())
}

fn save_state() {
    let path = match get_state_file_path_with_create_dir() {
        Some(p) => p,
        None => return,
    };
    info!("Saving state to {}", path.display());
    let servers = match RUNNING_SERVERS.lock() {
        Ok(g) => g,
        Err(p) => {
            error!("Mutex poisoned on save: {}", p);
            return;
        }
    };
    let serializable_servers: HashMap<String, RunningServer> = servers
        .iter()
        .map(|(id, rs)| {
            (
                id.clone(),
                RunningServer {
                    info: rs.info.clone(),
                    task_handle: None,
                },
            )
        })
        .collect();
    drop(servers);
    match bincode::serialize(&serializable_servers) {
        Ok(encoded) => {
            let temp_path = path.with_extension("tmp");
            match fs::write(&temp_path, encoded) {
                Ok(_) => {
                    #[cfg(unix)]
                    {
                        match fs::metadata(&temp_path) {
                            Ok(m) => {
                                let mut p = m.permissions();
                                p.set_mode(0o660);
                                let _ = fs::set_permissions(&temp_path, p);
                            }
                            Err(e) => warn!(
                                "Failed get/set temp state perms {}: {}",
                                temp_path.display(),
                                e
                            ),
                        }
                    }
                    if let Err(e) = fs::rename(&temp_path, &path) {
                        error!(
                            "Failed rename {} to {}: {}",
                            temp_path.display(),
                            path.display(),
                            e
                        );
                        let _ = fs::remove_file(&temp_path);
                    } else {
                        trace!("State saved.");
                    }
                }
                Err(e) => error!("Failed write temp state {}: {}", temp_path.display(), e),
            }
        }
        Err(e) => error!("Failed serialize state: {}", e),
    }
}

fn load_state() {
    let path = match get_state_file_path_with_create_dir() {
        Some(p) => p,
        None => return,
    };
    if !path.exists() {
        info!("State file {} not found.", path.display());
        return;
    }
    info!("Loading state from {}", path.display());
    match fs::read(&path) {
        Ok(encoded) => {
            match bincode::deserialize::<HashMap<String, RunningServer>>(&encoded) {
                Ok(loaded) => {
                    match RUNNING_SERVERS.lock() {
                        Ok(mut s) => {
                            *s = loaded;
                            info!("Loaded {} servers.", s.len());
                        }
                        Err(p) => error!("Mutex poisoned on load: {}", p),
                    }
                }
                Err(e) => {
                    error!("Failed deserialize {}: {}.", path.display(), e);
                    #[cfg(feature = "chrono")]
                    let bp = path.with_extension(format!(
                        "corrupted-{}",
                        chrono::Utc::now().timestamp()
                    ));
                    #[cfg(not(feature = "chrono"))]
                    let bp = path.with_extension("corrupted");
                    if let Err(re) = fs::rename(&path, &bp) {
                        error!(
                            "Failed backup {} to {}: {}",
                            path.display(),
                            bp.display(),
                            re
                        );
                    } else {
                        warn!("Corrupted file moved to {}", bp.display());
                    }
                }
            }
        }
        Err(e) => error!("Failed read {}: {}", path.display(), e),
    }
}

async fn process_command(command: Command, client_id: Uuid) -> Result<Response, CoreError> {
    let core_result = netter_core::execute_core_command(command.clone()).await;
    info!("[Client {}] Core result: {:?}", client_id, core_result);
    match core_result {
        CoreExecutionResult::CliResponse(core_response) => {
            match command {
                Command::StopServer { server_id } => {
                    info!("Handling StopServer: {}", server_id);
                    let mut servers = RUNNING_SERVERS
                        .lock()
                        .map_err(|_| CoreError::InternalError("Mutex poisoned".to_string()))?;
                    if let Some(srv) = servers.get_mut(&server_id) {
                        if let Some(h) = srv.task_handle.take() {
                            info!("Aborting task {}", server_id);
                            h.abort();
                            servers.remove(&server_id);
                            drop(servers);
                            save_state();
                            Ok(Response::ServerStopped(server_id))
                        } else {
                            warn!("Task handle missing for {}. Removing.", server_id);
                            servers.remove(&server_id);
                            drop(servers);
                            save_state();
                            Err(CoreError::OperationFailed(format!(
                                "Handle missing for {}, removed.",
                                server_id
                            )))
                        }
                    } else {
                        warn!("Not found: {}", server_id);
                        Err(CoreError::ServerNotFound(server_id))
                    }
                }
                Command::GetServerStatus { server_id } => {
                    info!("Handling GetServerStatus: {}", server_id);
                    let servers = RUNNING_SERVERS
                        .lock()
                        .map_err(|_| CoreError::InternalError("Mutex poisoned".to_string()))?;
                    if let Some(srv) = servers.get(&server_id) {
                        let mut info = srv.info.clone();
                        info.status = match srv.task_handle {
                            Some(ref h) if !h.is_finished() => "Running".to_string(),
                            Some(_) => "Stopped (Task Finished)".to_string(),
                            None => "Loaded (Unknown State)".to_string(),
                        };
                        Ok(Response::ServerStatus(info))
                    } else {
                        warn!("Not found: {}", server_id);
                        Err(CoreError::ServerNotFound(server_id))
                    }
                }
                Command::GetAllServersStatus => {
                    info!("Handling GetAllServersStatus.");
                    if !matches!(core_response, Response::Ok) {
                        warn!("Core returned non-Ok: {:?}", core_response);
                        return Ok(core_response);
                    }
                    let list: Vec<ServerInfo> = {
                        let servers = RUNNING_SERVERS
                            .lock()
                            .map_err(|_| CoreError::InternalError("Mutex poisoned".to_string()))?;
                        servers
                            .values()
                            .map(|rs| {
                                let mut i = rs.info.clone();
                                i.status = match rs.task_handle {
                                    Some(ref h) if !h.is_finished() => "Running".to_string(),
                                    Some(_) => "Stopped (Task Finished)".to_string(),
                                    None => "Loaded (Unknown State)".to_string(),
                                };
                                i
                            })
                            .collect()
                    };
                    info!("Found {} servers.", list.len());
                    Ok(Response::AllServersStatusReport(list))
                }
                _ => Ok(core_response),
            }
        }
        CoreExecutionResult::StartHttpServer {
            interpreter,
            tls_config,
        } => {
            debug!("Core returned StartHttpServer.");
            let server_id = Uuid::new_v4().to_string();

            let default_host = "127.0.0.1";
            let default_port: u16 = 9090;

            let (addr_str, port) = if let Some(config) = &interpreter.configuration {
                if config.config_type.eq_ignore_ascii_case("http") {
                    let host = if config.host.is_empty() {
                        warn!("Config block 'http' found but host is empty, using default '{}'", default_host);
                        default_host
                    } else {
                        &config.host
                    };

                    let port = config.port.parse::<u16>().unwrap_or_else(|_| {
                        warn!("Failed to parse port '{}' from config, using default {}", config.port, default_port);
                        default_port
                    });

                    info!("Using host '{}' and port {} from 'config' block.", host, port);
                    (host.to_string(), port)
                } else {
                    warn!("Config block found but type is not 'http' (is '{}'), using defaults.", config.config_type);
                    (default_host.to_string(), default_port)
                }
            } else {
                info!("No 'config' block found in configuration, using default host '{}' and port {}.", default_host, default_port);
                (default_host.to_string(), default_port)
            };

            let server_state = Arc::new(HttpCoreServer::from_interpreter(interpreter, tls_config));

            let socket_addr_str = format!("{}:{}", addr_str, port);
            info!(
                "Attempting to start HTTP server (ID: {}) on {}...",
                server_id, socket_addr_str
            );

            let task_handle = tokio::spawn({
                let id_c = server_id.clone();
                let state_c = server_state.clone();
                let addr_c = socket_addr_str.clone();
                async move {
                    http_core::run_hyper_server(addr_c, state_c, id_c).await;
                }
            });

            let server_info = ServerInfo {
                server_id: server_id.clone(),
                server_type: ServerType::Http,
                address: socket_addr_str,
                pid: None,
                status: "Starting".to_string(),
            };

             {
                let mut servers = RUNNING_SERVERS
                    .lock()
                    .map_err(|p| CoreError::InternalError(format!("Mutex poisoned on server insert: {}", p)))?;
                 servers.insert(
                    server_id.clone(),
                    RunningServer {
                        info: server_info.clone(),
                        task_handle: Some(task_handle),
                    },
                );
            }
            save_state();
            info!("Server {} added to running list and state saved.", server_id);
            Ok(Response::ServerStarted(server_info))
        }
    }
}

#[cfg(windows)]
mod windows_service_impl {
    use super::*;
    pub(crate) const SERVICE_NAME: &str = super::APPLICATION;
    const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
    const PIPE_NAME: &str = r"\\.\pipe\MyNetterServicePipe";
    define_windows_service!(ffi_service_main, service_main);

    pub fn service_main(arguments: Vec<OsString>) {
        let log_dir = &*LOG_PATH;
        if let Err(e) = fs::create_dir_all(log_dir) {
            eprintln!(
                "[{}] CRITICAL: Failed create log dir '{}': {}",
                SERVICE_NAME,
                log_dir.display(),
                e
            );
            report_service_error_status(101);
            std::process::exit(101);
        }
        if let Err(e) = netter_logger::init(Some(log_dir), LevelFilter::Info, LevelFilter::Trace) {
            eprintln!(
                "[{}] CRITICAL: Failed init logger: {}",
                SERVICE_NAME, e
            );
            report_service_error_status(100);
            std::process::exit(100);
        }
        info!(
            "Starting service {} (PID: {})...",
            SERVICE_NAME,
            std::process::id()
        );
        info!("Args: {:?}", arguments);
        info!("State file: {}", STATE_FILE_PATH.display());
        load_state();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        match run_service(arguments, shutdown_tx, shutdown_rx) {
            Ok(_) => info!("Service {} stopped.", SERVICE_NAME),
            Err(e) => {
                error!("Critical service error: {}", e);
                report_service_error_status(1);
                std::process::exit(1);
            }
        }
        info!("Service process {} finished.", SERVICE_NAME);
    }

    fn run_service(
        _args: Vec<OsString>,
        shutdown_tx: mpsc::Sender<()>,
        shutdown_rx: mpsc::Receiver<()>,
    ) -> WindowsServiceResult<()> {
        info!("Initializing service logic...");
        let (err_tx_async, err_rx_async) =
            tokio_mpsc::channel::<Box<dyn StdError + Send + Sync>>(10);
        let status_handle = match service_control_handler::register(SERVICE_NAME, move |ev| match ev
        {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                info!("SCM signal: {:?}", ev);
                let _ = shutdown_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }) {
            Ok(h) => h,
            Err(e) => {
                error!("SCM handler register error: {}", e);
                return Err(e);
            }
        };
        info!("SCM handler registered.");
        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::StartPending,
            controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 1,
            wait_hint: Duration::from_secs(5),
            process_id: None,
        })?;
        info!("Status: StartPending");

        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("netter-worker")
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                error!("Tokio runtime error: {}", e);
                status_handle.set_service_status(ServiceStatus {
                    service_type: SERVICE_TYPE,
                    current_state: ServiceState::Stopped,
                    controls_accepted: ServiceControlAccept::empty(),
                    exit_code: ServiceExitCode::ServiceSpecific(2),
                    checkpoint: 0,
                    wait_hint: Duration::default(),
                    process_id: None,
                })?;
                return Err(windows_service::Error::Winapi(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Tokio runtime: {}", e),
                )));
            }
        };
        info!("Tokio runtime created.");

        let async_handle = rt.spawn(run_async_server(err_tx_async.clone()));
        info!("Async IPC task spawned.");

        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;
        info!("Status: Running. IPC on '{}'", PIPE_NAME);

        #[allow(unused_assignments)]
        let mut stop_reason = "Unknown".to_string();
        let mut exit_code = ServiceExitCode::Win32(0);
        let mut err_rx = err_rx_async;
        loop {
            match err_rx.try_recv() {
                Ok(e) => {
                    error!("Async error: {}", e);
                    stop_reason = format!("Async error: {}", e);
                    exit_code = ServiceExitCode::ServiceSpecific(3);
                    break;
                }
                Err(tokio_mpsc::error::TryRecvError::Empty) => {}
                Err(_) => {
                    error!("Async err chan disconnected!");
                    stop_reason = "Async err chan disconnected".into();
                    exit_code = ServiceExitCode::ServiceSpecific(6);
                    break;
                }
            }
            match shutdown_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(_) => {
                    info!("SCM shutdown signal.");
                    stop_reason = "SCM signal".into();
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if async_handle.is_finished() {
                        error!("Async task finished unexpectedly!");
                        stop_reason = "Async task finished".into();
                        exit_code = ServiceExitCode::ServiceSpecific(4);
                        break;
                    }
                    continue;
                }
                Err(_) => {
                    error!("SCM chan disconnected!");
                    stop_reason = "SCM chan disconnected".into();
                    exit_code = ServiceExitCode::ServiceSpecific(5);
                    break;
                }
            }
        }

        info!("Shutting down (Reason: {})...", stop_reason);
        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::StopPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 1,
            wait_hint: Duration::from_secs(15),
            process_id: None,
        })?;
        info!("Status: StopPending (1)");

        save_state();

        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::StopPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 2,
            wait_hint: Duration::from_secs(10),
            process_id: None,
        })?;

        info!("Status: StopPending (2)");
        info!("Shutting down Tokio (5s)...");

        rt.shutdown_timeout(Duration::from_secs(5));

        info!("Tokio shut down.");

        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code,
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;

        info!("Status: Stopped (Code: {:?})", exit_code);

        if matches!(exit_code, ServiceExitCode::Win32(0)) {
            Ok(())
        } else {
            Err(windows_service::Error::Winapi(io::Error::new(
                io::ErrorKind::Other,
                stop_reason,
            )))
        }
    }

    async fn run_async_server(error_tx: tokio_mpsc::Sender<Box<dyn StdError + Send + Sync>>) {
        let mut server = match create_pipe_server() {
            Ok(s) => s,
            Err(e) => {
                let emsg = format!("IPC create: {}", e);
                error!("{}", emsg);
                let _ = error_tx.send(Box::new(CoreError::IoError(emsg))).await;
                return;
            }
        };
        info!("IPC listening on '{}'", PIPE_NAME);
        loop {
            match server.connect().await {
                Ok(_) => {
                    trace!("Client connected.");
                    let client_pipe = server;
                    server = match create_pipe_server() {
                        Ok(s) => s,
                        Err(e) => {
                            let emsg = format!("IPC next: {}", e);
                            error!("{}", emsg);
                            let _ = error_tx.send(Box::new(CoreError::IoError(emsg))).await;
                            break;
                        }
                    };
                    trace!("New IPC instance.");
                    tokio::spawn(async move {
                        handle_client_windows(client_pipe).await;
                    });
                }
                Err(e) => {
                    error!("IPC accept: {}. Recreating...", e);

                    tokio::time::sleep(Duration::from_secs(2)).await;
                    server = match create_pipe_server() {
                        Ok(s) => s,
                        Err(er) => {
                            let emsg = format!("IPC recreate: {}", er);
                            error!("{}", emsg);
                            let _ = error_tx.send(Box::new(CoreError::IoError(emsg))).await;
                            break;
                        }
                    };
                    warn!("IPC server recreated.");
                }
            }
        }
        warn!("Async IPC loop finished.");
    }

    fn create_pipe_server() -> io::Result<NamedPipeServer> {
        ServerOptions::new()
            .pipe_mode(PipeMode::Message)
            .first_pipe_instance(false)
            .reject_remote_clients(true)
            .create(PIPE_NAME)
    }

    async fn handle_client_windows(mut pipe: NamedPipeServer) {
        let client_id = Uuid::new_v4();
        trace!("[Client {}] Start (WinPipe).", client_id);

        let command_result: Result<Command, CoreError> = async {
            let mut size_buf = [0u8; 4];
            pipe.read_exact(&mut size_buf)
                .await
                .map_err(|e| CoreError::IoError(format!("Pipe read size: {}", e)))?;
            let command_size = u32::from_be_bytes(size_buf) as usize;
            trace!("[Client {}] Command size header: {}", client_id, command_size);

            const MAX_CMD_SIZE: usize = 1 * 1024 * 1024;
            if command_size > MAX_CMD_SIZE {
                return Err(CoreError::InvalidInput(format!(
                    "Command size {} exceeds limit {}",
                    command_size, MAX_CMD_SIZE
                )));
            }
            if command_size == 0 {
                 return Err(CoreError::InvalidInput("Received zero command size".to_string()));
            }

            let mut command_buffer = vec![0u8; command_size];
            pipe.read_exact(&mut command_buffer)
                .await
                .map_err(|e| CoreError::IoError(format!("Pipe read body ({} bytes): {}", command_size, e)))?;

            trace!("[Client {}] Read {} command bytes.", client_id, command_buffer.len());

            bincode::deserialize(&command_buffer)
                .map_err(|e| CoreError::DeserializationError(format!("Command parse: {}", e)))

        }.await;

        let response = match command_result {
             Ok(command) => {
                trace!("[Client {}] Received command: {:?}", client_id, command);
                process_command(command, client_id).await
             }
             Err(e) => {
                error!("[Client {}] Error reading/parsing command: {}", client_id, e);
                Err(e)
             }
        };

        let final_response = match response {
            Ok(r) => r,
            Err(e) => {
                error!("[Client {}] Processing error: {}", client_id, e);
                Response::Error(e)
            }
        };

        match bincode::serialize(&final_response) {
            Ok(response_bytes) => {
                let response_size = response_bytes.len() as u32;
                trace!(
                    "[Client {}] Sending response ({} bytes): {:?}",
                    client_id,
                    response_size,
                    final_response
                );

                if let Err(e) = pipe.write_all(&response_size.to_be_bytes()).await {
                    error!("[Client {}] Pipe write response size error: {}", client_id, e);
                } else {
                    if let Err(e) = pipe.write_all(&response_bytes).await {
                        error!("[Client {}] Pipe write response body error: {}", client_id, e);
                    } else {
                        if let Err(e) = pipe.flush().await {
                             warn!("[Client {}] Pipe flush error: {}", client_id, e);
                        } else {
                            trace!("[Client {}] Response sent successfully.", client_id);
                        }
                    }
                }
            }
            Err(e) => {
                error!("[Client {}] Failed to serialize final response: {}", client_id, e);
                let error_resp = Response::Error(CoreError::SerializationError(format!("Failed to serialize service response: {}", e)));
                 if let Ok(error_bytes) = bincode::serialize(&error_resp) {
                     let error_size = error_bytes.len() as u32;
                     if pipe.write_all(&error_size.to_be_bytes()).await.is_ok() {
                         let _ = pipe.write_all(&error_bytes).await;
                         let _ = pipe.flush().await;
                     }
                 }
            }
        }

        trace!("[Client {}] Finish (WinPipe).", client_id);
    }

    fn report_service_error_status(code: u32) {
        if let Ok(h) =
            service_control_handler::register(SERVICE_NAME, |_| ServiceControlHandlerResult::NoError)
        {
            let _ = h.set_service_status(ServiceStatus {
                service_type: SERVICE_TYPE,
                current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::ServiceSpecific(code),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            });
            warn!("Reported err {} to SCM.", code);
        } else {
            eprintln!(
                "[{}] Failed report err {} to SCM.",
                SERVICE_NAME, code
            );
        }
    }

    #[cfg(windows)]
    pub fn main_entry() -> windows_service::Result<()> {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)
    }
}

#[cfg(unix)]
mod unix_daemon_impl {
    use super::*;
    const SOCKET_DIR_FALLBACK: &str = "/tmp/netterd";
    const SOCKET_NAME: &str = "netterd.sock";

    pub async fn daemon_main() -> Result<(), Box<dyn StdError>> {
        let _log_file = &*LOG_PATH;
        if let Err(e) = netter_logger::init(None::<PathBuf>, LevelFilter::Trace, LevelFilter::Trace) {
            eprintln!("CRITICAL: Failed init logger: {}", e);
            std::process::exit(100);
        }

        info!("Starting daemon {}...", APPLICATION);
        info!("Socket path: {}", get_socket_path().display());
        info!("State file: {}", STATE_FILE_PATH.display());

        load_state();

        let (shutdown_tx, mut shutdown_rx) = tokio_mpsc::channel::<()>(1);

        let signals_task = tokio::spawn(handle_signals(shutdown_tx.clone()));
        let ipc_server_task = tokio::spawn(run_ipc_server(shutdown_tx));
        info!("Daemon {} started.", APPLICATION);

        let shutdown_reason: String;
        tokio::select! {
            _ = shutdown_rx.recv() => { shutdown_reason = "Shutdown signal".into(); info!("Shutdown signal."); }
            res = ipc_server_task => { match res { Ok(_) => shutdown_reason="IPC task finished".into(), Err(e)=>shutdown_reason=format!("IPC task err: {}",e) }; error!("{}", shutdown_reason); }
            res = signals_task => { match res { Ok(_) => shutdown_reason="Signal task finished".into(), Err(e)=>shutdown_reason=format!("Signal task err: {}",e) }; error!("{}", shutdown_reason); }
        }

        info!("Shutting down (Reason: {})...", shutdown_reason);

        save_state();

        info!("Stopping servers...");
        let ids: Vec<String> = match RUNNING_SERVERS.lock() {
            Ok(g) => g.keys().cloned().collect(),
            Err(_) => {
                error!("Mutex poisoned on shutdown.");
                vec![]
            }
        };
        if !ids.is_empty() {
            let mut g = RUNNING_SERVERS.lock().unwrap();
            let mut h = Vec::new();
            for id in ids {
                if let Some(s) = g.get_mut(&id) {
                    if let Some(t) = s.task_handle.take() {
                        info!("Stopping {}...", id);
                        t.abort();
                        h.push(t);
                    }
                }
            }
            drop(g);
            for t in h {
                let _ = tokio::time::timeout(Duration::from_secs(5), t).await;
            }
            info!("Servers stopped.");
        } else {
            info!("No servers to stop.");
        }

        let sp = get_socket_path();
        if sp.exists() {
            info!("Removing socket {}", sp.display());
            let _ = fs::remove_file(&sp);
        }
        info!("Daemon {} shut down.", APPLICATION);
        Ok(())
    }

    fn get_socket_path() -> PathBuf {
        let rd = Path::new("/run").join(APPLICATION.to_lowercase());
        if fs::create_dir_all(&rd).is_ok() {
            match fs::metadata(&rd) {
                Ok(m) => {
                    let mut p = m.permissions();
                    p.set_mode(0o770);
                    let _ = fs::set_permissions(&rd, p);
                }
                Err(e) => warn!("Perms err {}: {}", rd.display(), e),
            }
            rd.join(SOCKET_NAME)
        } else {
            warn!("Fallback {}", SOCKET_DIR_FALLBACK);
            let fd = Path::new(SOCKET_DIR_FALLBACK);
            let _ = fs::create_dir_all(fd);
            fd.join(SOCKET_NAME)
        }
    }

    async fn handle_signals(tx: tokio_mpsc::Sender<()>) {
        let mut si = match signal(SignalKind::interrupt()) {
            Ok(s) => s,
            Err(e) => {
                error!("SIGINT fail: {}", e);
                return;
            }
        };
        let mut st = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                error!("SIGTERM fail: {}", e);
                return;
            }
        };
        tokio::select! {
            _=si.recv()=>{info!("SIGINT.");},
            _=st.recv()=>{info!("SIGTERM.");},
        };
        let _ = tx.send(()).await;
        info!("Signal handling done.");
    }

    async fn run_ipc_server(tx: tokio_mpsc::Sender<()>) {
        let sp = get_socket_path();
        if sp.exists() {
            warn!("Remove old socket {}", sp.display());
            if let Err(e) = fs::remove_file(&sp) {
                error!("Remove fail: {}.", e);
                let _ = tx.send(()).await;
                return;
            }
        }
        let l = match UnixListener::bind(&sp) {
            Ok(l) => l,
            Err(e) => {
                error!("Bind {}: {}.", sp.display(), e);
                let _ = tx.send(()).await;
                return;
            }
        };
        match fs::metadata(&sp) {
            Ok(m) => {
                let mut p = m.permissions();
                p.set_mode(0o666);
                if let Err(e) = fs::set_permissions(&sp, p) {
                    warn!("Perms err: {}", e);
                } else {
                    info!("Socket perms 0666.");
                }
            }
            Err(e) => warn!("Meta err: {}", e),
        };
        info!("IPC listening {}", sp.display());
        loop {
            match l.accept().await {
                Ok((s, _)) => {
                    trace!("Client.");
                    tokio::spawn(async move {
                        handle_client_unix(s).await;
                    });
                }
                Err(e) => {
                    error!("Accept: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn handle_client_unix(mut stream: UnixStream) {
        let cid = Uuid::new_v4();
        trace!("[Client {}] Start (Unix).", cid);
        let res: Result<Response, CoreError> = async {
            let mut sb = [0u8; 4];
            stream
                .read_exact(&mut sb)
                .await
                .map_err(|e| CoreError::IoError(format!("Read size: {}", e)))?;
            let ms = u32::from_be_bytes(sb) as usize;
            trace!("[Client {}] Size: {}", cid, ms);
            const MAX: usize = 10 * 1024 * 1024;
            if ms > MAX {
                return Err(CoreError::IoError("Too large".into()));
            }
            let mut b = vec![0; ms];
            stream
                .read_exact(&mut b)
                .await
                .map_err(|e| CoreError::IoError(format!("Read body: {}", e)))?;
            trace!("[Client {}] Read {}b.", cid, ms);
            let cmd: Command = bincode::deserialize(&b)
                .map_err(|e| CoreError::DeserializationError(format!("Parse: {}", e)))?;
            process_command(cmd, cid).await
        }
        .await;
        match res {
            Ok(r) => {
                match bincode::serialize(&r) {
                    Ok(b) => {
                        let s = b.len() as u32;
                        trace!("[Client {}] Send resp ({}b).", cid, b.len());
                        if let Err(e) = stream.write_all(&s.to_be_bytes()).await {
                            error!("Write size: {}", e);
                            return;
                        }
                        if let Err(e) = stream.write_all(&b).await {
                            error!("Write body: {}", e);
                        } else {
                            trace!("Resp sent.");
                        }
                    }
                    Err(e) => {
                        error!("Serialize err: {}", e);
                        let er = Response::Error(CoreError::SerializationError(e.to_string()));
                        if let Ok(eb) = bincode::serialize(&er) {
                            let s = eb.len() as u32;
                            if stream.write_all(&s.to_be_bytes()).await.is_ok() {
                                let _ = stream.write_all(&eb).await;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Processing err: {}", e);
                let er = Response::Error(e);
                if let Ok(eb) = bincode::serialize(&er) {
                    let s = eb.len() as u32;
                    if stream.write_all(&s.to_be_bytes()).await.is_ok() {
                        let _ = stream.write_all(&eb).await;
                    }
                }
            }
        }
        trace!("[Client {}] Finish (Unix).", cid);
    }
}

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    windows_service_impl::main_entry()
}

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    unix_daemon_impl::daemon_main().await
}

#[cfg(not(any(windows, unix)))]
fn main() {
    eprintln!("Error: Unsupported OS.");
    std::process::exit(1);
}