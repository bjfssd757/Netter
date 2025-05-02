#![windows_subsystem = "windows"]

use std::{
    collections::HashMap,
    ffi::OsString,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
    error::Error as StdError,
};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, Result,
};
use log::{error, info, trace, warn, LevelFilter, debug};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions},
    task::JoinHandle,
};
use uuid::Uuid;
use netter_core::{
    Command, CoreError, CoreExecutionResult, Response, ServerInfo, ServerType, servers
};
use hyper_util::rt::TokioIo;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use tokio_rustls::TlsAcceptor;

const SERVICE_NAME: &str = "NetterService";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
const PIPE_NAME: &str = r"\\.\pipe\MyNetterServicePipe";
const SERVICE_LOG_DIR: &str = "C:\\ProgramData\\NetterService\\Logs";

#[derive(Debug)]
struct RunningServer {
    info: ServerInfo,
    task_handle: JoinHandle<()>,
}

lazy_static::lazy_static! {
    static ref RUNNING_SERVERS: Arc<Mutex<HashMap<String, RunningServer>>> = Arc::new(Mutex::new(HashMap::new()));
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(arguments: Vec<OsString>) {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    if let Err(e) = std::fs::create_dir_all(SERVICE_LOG_DIR) {
         eprintln!("[{}] CRITICAL: Failed to create log directory '{}': {}", SERVICE_NAME, SERVICE_LOG_DIR, e);
         report_service_error_status(101); std::process::exit(101);
    }
    if let Err(e) = netter_logger::init(Some(SERVICE_LOG_DIR), LevelFilter::Info, LevelFilter::Trace) {
        eprintln!("[{}] CRITICAL: Failed to initialize service logger: {}", SERVICE_NAME, e);
        report_service_error_status(100); std::process::exit(100);
    }

    info!("Starting service {} (PID: {})...", SERVICE_NAME, std::process::id());
    info!("Service arguments: {:?}", arguments);

    match run_service(arguments, shutdown_tx, shutdown_rx) {
        Ok(_) => info!("Service {} stopped successfully.", SERVICE_NAME),
        Err(e) => {
            error!("Critical service error {}: {}", SERVICE_NAME, e);
            report_service_error_status(1); std::process::exit(1);
        }
    }
    info!("Service process {} finished.", SERVICE_NAME);
}

fn run_service(
    _arguments: Vec<OsString>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: mpsc::Receiver<()>,
) -> Result<()> {
    info!("Initializing service logic {}...", SERVICE_NAME);
    let (error_tx, error_rx) = mpsc::channel::<Box<dyn StdError + Send + Sync>>();

    let status_handle = match service_control_handler::register(SERVICE_NAME, move |control_event| {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                info!("Received {:?} signal from SCM.", control_event);
                let _ = shutdown_tx.send(()); ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    }) {
        Ok(handle) => handle,
        Err(e) => { error!("CRITICAL: Failed to register SCM handler: {}", e); return Err(e); }
    };
    info!("SCM control handler registered successfully.");

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE, current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0), checkpoint: 1,
        wait_hint: Duration::from_secs(5), process_id: None,
    })?;
    info!("Service status set to StartPending");

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all().thread_name("netter-worker").build() {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to create Tokio runtime: {}", e);
            status_handle.set_service_status(ServiceStatus {
                service_type: SERVICE_TYPE, current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::empty(), exit_code: ServiceExitCode::ServiceSpecific(2),
                checkpoint: 0, wait_hint: Duration::default(), process_id: None,
            })?;
            return Err(windows_service::Error::Winapi(std::io::Error::new(
                std::io::ErrorKind::Other, format!("Failed to create Tokio runtime: {}", e),
            )));
        }
    };
    info!("Tokio runtime created successfully.");

    let async_main_handle = rt.spawn(run_async_server(error_tx.clone()));
    info!("Async IPC server task spawned in Tokio runtime.");

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE, current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0), checkpoint: 0,
        wait_hint: Duration::default(), process_id: None,
    })?;
    info!("Service status set to Running. Waiting for IPC connections on '{}'", PIPE_NAME);

    let mut service_stop_reason = "Unknown".to_string();
    let mut final_exit_code = ServiceExitCode::Win32(0);
    loop {
        match error_rx.try_recv() {
            Ok(err) => {
                error!("Received critical error from async task: {}", err);
                service_stop_reason = format!("Async task error: {}", err);
                final_exit_code = ServiceExitCode::ServiceSpecific(3); break;
            }
            Err(mpsc::TryRecvError::Empty) => {
                match shutdown_rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(_) => {
                        info!("Received shutdown signal from SCM in main loop.");
                        service_stop_reason = "Shutdown signal from SCM".to_string(); break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if async_main_handle.is_finished() {
                            error!("Async IPC server task finished unexpectedly!");
                            match rt.block_on(async { async_main_handle.await }) {
                                Ok(_) => service_stop_reason = "Async IPC server task finished (Ok)".to_string(),
                                Err(e) => service_stop_reason = format!("Async IPC server task finished (Panic/Error): {}", e),
                            }
                            final_exit_code = ServiceExitCode::ServiceSpecific(4); break;
                        }
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        error!("SCM signal channel disconnected unexpectedly!");
                        service_stop_reason = "SCM signal channel disconnected".to_string();
                        final_exit_code = ServiceExitCode::ServiceSpecific(5); break;
                    }
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                 error!("Async error channel disconnected!");
                 if async_main_handle.is_finished() {
                     warn!("Async error channel disconnected and task finished. Stop reason: {}", service_stop_reason);
                     if service_stop_reason == "Unknown" { service_stop_reason = "Async error channel disconnected and task finished".to_string(); }
                 } else {
                     service_stop_reason = "Async error channel disconnected unexpectedly while task running".to_string();
                     final_exit_code = ServiceExitCode::ServiceSpecific(6);
                 }
                 break;
            }
        }
    }

    info!("Starting service shutdown process (Reason: {})...", service_stop_reason);
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE, current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(), exit_code: ServiceExitCode::Win32(0),
        checkpoint: 1, wait_hint: Duration::from_secs(15), process_id: None,
    })?;
    info!("Service status set to StopPending");

    info!("Stopping all running servers...");
    let server_ids_to_stop: Vec<String> = RUNNING_SERVERS.lock().unwrap().keys().cloned().collect();
    if !server_ids_to_stop.is_empty() {
        let mut servers = RUNNING_SERVERS.lock().unwrap();
        for server_id in server_ids_to_stop {
            if let Some(running_server) = servers.remove(&server_id) {
                info!("Stopping server ID: {} (Type: {:?}, Addr: {})",
                      running_server.info.server_id, running_server.info.server_type, running_server.info.address);
                running_server.task_handle.abort();
            }
        }
        info!("Abort signal sent to all server tasks.");
    } else { info!("No active servers to stop."); }

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE, current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(), exit_code: ServiceExitCode::Win32(0),
        checkpoint: 2, wait_hint: Duration::from_secs(10), process_id: None,
    })?;

    info!("Shutting down Tokio runtime (timeout 5s)...");
    rt.shutdown_timeout(Duration::from_secs(5));
    info!("Tokio runtime shut down.");

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE, current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(), exit_code: final_exit_code,
        checkpoint: 0, wait_hint: Duration::default(), process_id: None,
    })?;
    info!("Service status set to Stopped (Exit code: {:?})", final_exit_code);

    if matches!(final_exit_code, ServiceExitCode::Win32(0)) { Ok(()) }
    else { Err(windows_service::Error::Winapi(std::io::Error::new(std::io::ErrorKind::Other, service_stop_reason))) }
}

async fn run_async_server(error_tx: mpsc::Sender<Box<dyn StdError + Send + Sync>>) {
    let mut server = match create_pipe_server() {
        Ok(s) => s,
        Err(e) => {
            let err_msg = format!("Critical: Failed to create IPC server '{}': {}", PIPE_NAME, e);
            error!("{}", err_msg);
            let _ = error_tx.send(Box::new(CoreError::IoError(err_msg))); return;
        }
    };
    info!("IPC server listening on '{}'", PIPE_NAME);

    loop {
        match server.connect().await {
            Ok(_) => {
                trace!("Client connected to IPC pipe.");
                let client_pipe = server;

                server = match create_pipe_server() {
                    Ok(s) => s,
                    Err(e) => {
                        let err_msg = format!("Failed to create next IPC server instance '{}': {}", PIPE_NAME, e);
                        error!("{}", err_msg);
                        let _ = error_tx.send(Box::new(CoreError::IoError(err_msg))); break;
                    }
                };
                trace!("New IPC server instance created for next client.");

                let client_error_tx = error_tx.clone();
                tokio::spawn(async move { handle_client(client_pipe, client_error_tx).await; });
            }
            Err(e) => {
                error!("Error awaiting client connection on IPC '{}': {}. Recreating server in 2s...", PIPE_NAME, e);
                tokio::time::sleep(Duration::from_secs(2)).await;
                server = match create_pipe_server() {
                    Ok(s) => s,
                    Err(e_retry) => {
                        let err_msg = format!("Critical: Failed to recreate IPC server '{}': {}", PIPE_NAME, e_retry);
                        error!("{}", err_msg);
                        let _ = error_tx.send(Box::new(CoreError::IoError(err_msg))); break;
                    }
                };
                warn!("IPC server '{}' recreated successfully after error.", PIPE_NAME);
            }
        }
    }
    warn!("Async IPC server loop finished.");
}

fn create_pipe_server() -> std::io::Result<NamedPipeServer> {
     ServerOptions::new()
        .pipe_mode(PipeMode::Message)
        .first_pipe_instance(false)
        .reject_remote_clients(true)
        .create(PIPE_NAME)
}


async fn handle_client(
    mut pipe: NamedPipeServer,
    error_tx: mpsc::Sender<Box<dyn StdError + Send + Sync>>,
) {
    let client_id = Uuid::new_v4();
    trace!("[Client {}] Start processing.", client_id);

    let result = async {
        let mut buffer = vec![0u8; 4096];
        let mut total_bytes_read = 0;
        let mut read_attempts = 0;
        const MAX_READ_ATTEMPTS: u32 = 5;
        const READ_RETRY_DELAY: Duration = Duration::from_millis(50);

        loop {
            read_attempts += 1;
            trace!("[Client {}] Read attempt #{}", client_id, read_attempts);

            match pipe.read(&mut buffer).await {
                Ok(bytes_read) => {
                    trace!("[Client {}] Read {} bytes from pipe.", client_id, bytes_read);
                    if bytes_read > 0 {
                        total_bytes_read = bytes_read;
                        break;
                    } else {
                        if read_attempts >= MAX_READ_ATTEMPTS {
                            warn!("[Client {}] Read 0 bytes after {} attempts, assuming client disconnected.", client_id, read_attempts);
                            return Err(CoreError::IoError("Client disconnected before sending data".to_string()));
                        } else {
                             warn!("[Client {}] Read 0 bytes on attempt {}, retrying after delay...", client_id, read_attempts);
                             tokio::time::sleep(READ_RETRY_DELAY).await;
                        }
                    }
                }
                Err(e) => {
                    error!("[Client {}] Pipe read error: {}", client_id, e);
                    return Err(CoreError::IoError(format!("Pipe read error: {}", e)));
                }
            }
        }

        let command: Command = bincode::deserialize(&buffer[..total_bytes_read])
            .map_err(|e| CoreError::DeserializationError(format!("Failed to parse command ({} bytes): {}", total_bytes_read, e)))?;
        info!("[Client {}] Received command: {:?}", client_id, command);

        let core_result = netter_core::execute_core_command(command.clone()).await;
        info!("[Client {}] Core execution result: {:?}", client_id, core_result);

        let response_to_cli = match core_result {
            CoreExecutionResult::CliResponse(mut core_cli_response) => {
                debug!("[Client {}] Core returned CLI response: {:?}", client_id, core_cli_response);
                match command {
                    Command::StopServer { server_id } => {
                         info!("[Service] Handling StopServer for ID: {}", server_id);
                         let mut servers = RUNNING_SERVERS.lock().unwrap();
                         if let Some(running_server) = servers.remove(&server_id) {
                             info!("[Service] Aborting task for server {}", server_id);
                             running_server.task_handle.abort();
                             if matches!(core_cli_response, Response::Ok) { Response::ServerStopped(server_id) }
                             else { core_cli_response }
                         } else {
                             warn!("[Service] Server ID {} not found for StopServer.", server_id);
                             if !matches!(core_cli_response, Response::Error(_)) { Response::Error(CoreError::ServerNotFound(server_id)) }
                             else { core_cli_response }
                         }
                    }
                    Command::GetServerStatus { server_id } => {
                         info!("[Service] Handling GetServerStatus for ID: {}", server_id);
                         let servers = RUNNING_SERVERS.lock().unwrap();
                         if let Some(running_server) = servers.get(&server_id) {
                             let mut info = running_server.info.clone();
                             info.status = if running_server.task_handle.is_finished() { "Stopped (Task Finished)".to_string() } else { "Running".to_string() };
                             Response::ServerStatus(info)
                         } else {
                             warn!("[Service] Server ID {} not found for GetServerStatus.", server_id);
                             if !matches!(core_cli_response, Response::Error(_)) { Response::Error(CoreError::ServerNotFound(server_id)) }
                             else { core_cli_response }
                         }
                    },
                    Command::GetAllServersStatus => {
                        info!("[Service] Handling GetAllServersStatus.");
                        if matches!(core_cli_response, Response::Ok) {
                            let servers_list: Vec<ServerInfo> = {
                                let servers = RUNNING_SERVERS.lock().unwrap();
                                servers.values().map(|s| s.info.clone()).collect()
                            };
                            info!("[Service] Found {} running servers.", servers_list.len());
                            Response::AllServersStatusReport(servers_list)
                        } else {
                            warn!("[Service] GetAllServersStatus: Core returned unexpected response: {:?}", core_cli_response);
                            core_cli_response
                        }
                    },
                    _ => core_cli_response,
                }
            }
            CoreExecutionResult::StartHttpServer { interpreter, tls_config } => {
                debug!("[Client {}] Core returned StartHttpServer data.", client_id);
                let server_id = Uuid::new_v4().to_string();
                let server_config_core = servers::http_core::Server::from_interpreter(interpreter, tls_config);
                let server_state = Arc::new(server_config_core);
                let (addr_str, port) = {
                    let interp_guard = server_state.interpreter.as_ref()
                        .ok_or_else(|| CoreError::InternalError("Interpreter missing in server state".to_string()))?
                        .lock()
                        .map_err(|_| CoreError::InternalError("Interpreter mutex poisoned".to_string()))?;
                    let host = "127.0.0.1".to_string();
                    let port_u16 = 9090;
                    (host, port_u16)
                };
                let socket_addr_str = format!("{}:{}", addr_str, port);
                info!("[Service] Starting HTTP server (ID: {}) on {}...", server_id, socket_addr_str);
                let task_handle = tokio::spawn({
                    let server_id_clone = server_id.clone();
                    let server_state_clone = server_state.clone();
                    let socket_addr_str_clone = socket_addr_str.clone();
                    let error_tx_http = error_tx.clone();
                    async move {
                        if let Ok(socket_addr) = socket_addr_str_clone.parse::<std::net::SocketAddr>() {
                             match tokio::net::TcpListener::bind(socket_addr).await {
                                 Ok(listener) => {
                                     info!("[HTTP Server ID: {}] Listening on {}", server_id_clone, socket_addr);
                                     loop {
                                        match listener.accept().await {
                                            Ok((tcp_stream, _remote_addr)) => {
                                                let server_state_clone2 = server_state_clone.clone();
                                                let server_id_clone2 = server_id_clone.clone();
                                                if server_state_clone2.is_tls_enabled() {
                                                    if let Some(rustls_conf_arc) = server_state_clone2.rustls_config.clone() {
                                                        let acceptor = TlsAcceptor::from(rustls_conf_arc);
                                                        tokio::spawn(async move {
                                                            match acceptor.accept(tcp_stream).await {
                                                                Ok(tls_stream) => {
                                                                    let io = TokioIo::new(tls_stream);
                                                                    let service = service_fn(move |req| {
                                                                        servers::http_core::handle_http_request(req, server_state_clone2.clone())
                                                                    });
                                                                     if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                                                                            error!("[HTTP Server ID: {}] TLS connection error: {}", server_id_clone2, err);
                                                                        }
                                                                },
                                                                Err(e) => error!("[HTTP Server ID: {}] TLS handshake error: {}", server_id_clone2, e),
                                                            }
                                                        });
                                                    } else { error!("[HTTP Server ID: {}] TLS enabled but rustls config missing!", server_id_clone2); }
                                                } else {
                                                    let io = TokioIo::new(tcp_stream);
                                                    let service = service_fn(move |req| {
                                                        servers::http_core::handle_http_request(req, server_state_clone2.clone())
                                                    });
                                                    tokio::spawn(async move {
                                                        if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                                                                error!("[HTTP Server ID: {}] HTTP connection error: {}", server_id_clone2, err);
                                                            }
                                                    });
                                                }
                                            },
                                            Err(e) => { error!("[HTTP Server ID: {}] Accept error: {}. Pausing...", server_id_clone, e); tokio::time::sleep(Duration::from_secs(1)).await; }
                                        }
                                     }
                                 },
                                 Err(e) => {
                                     let err_msg = format!("Failed to bind HTTP server (ID: {}) on {}: {}", server_id_clone, socket_addr, e);
                                     error!("[Service] CRITICAL ERROR: {}", err_msg);
                                     let _ = error_tx_http.send(Box::new(CoreError::IoError(err_msg)));
                                 },
                             }
                        } else {
                             let err_msg = format!("Invalid address for HTTP server (ID: {}): {}", server_id_clone, socket_addr_str_clone);
                             error!("{}", err_msg);
                             let _ = error_tx_http.send(Box::new(CoreError::InvalidInput(err_msg)));
                        }
                        warn!("[HTTP Server ID: {}] Server task finished.", server_id_clone);
                    }
                });
                let server_info = ServerInfo {
                    server_id: server_id.clone(), server_type: ServerType::Http,
                    address: socket_addr_str, pid: None, status: "Running".to_string(),
                };
                RUNNING_SERVERS.lock().unwrap().insert(server_id.clone(), RunningServer { info: server_info.clone(), task_handle });
                info!("[Service] Server {} added to running list.", server_id);
                Response::ServerStarted(server_info)
            }
        };
        Ok(response_to_cli)
    }.await;

    match result {
        Ok(response_to_cli) => {
            match bincode::serialize(&response_to_cli) {
                Ok(response_bytes) => {
                    trace!("[Client {}] Serialized response ({} bytes): {:?}", client_id, response_bytes.len(), response_to_cli);
                    if let Err(e) = pipe.write_all(&response_bytes).await {
                        error!("[Client {}] Error sending response: {}", client_id, e);
                    } else if let Err(e) = pipe.flush().await {
                        warn!("[Client {}] Error flushing pipe: {}", client_id, e);
                    } else {
                        trace!("[Client {}] Response sent successfully.", client_id);
                    }
                }
                Err(e) => {
                    error!("[Client {}] CRITICAL: Failed to serialize response: {}. Original response: {:?}", client_id, e, response_to_cli);
                    let error_response = Response::Error(CoreError::SerializationError(format!("Service failed to serialize response: {}", e)));
                    if let Ok(err_bytes) = bincode::serialize(&error_response) {
                        let _ = pipe.write_all(&err_bytes).await; let _ = pipe.flush().await;
                    }
                    let _ = error_tx.send(Box::new(CoreError::SerializationError(format!("Failed to serialize response: {}", e))));
                }
            }
        }
        Err(core_error) => {
             error!("[Client {}] Error during processing: {}", client_id, core_error);
             let response_to_cli = Response::Error(core_error);
             if let Ok(response_bytes) = bincode::serialize(&response_to_cli) {
                 if let Err(e) = pipe.write_all(&response_bytes).await {
                     error!("[Client {}] Error sending error response: {}", client_id, e);
                 } else { let _ = pipe.flush().await; }
             } else {
                  error!("[Client {}] CRITICAL: Failed to serialize error response itself!", client_id);
                  let _ = error_tx.send(Box::new(CoreError::SerializationError("Failed to serialize error response".to_string())));
             }
        }
    }

    trace!("[Client {}] Finished processing.", client_id);
}


fn report_service_error_status(error_code: u32) {
    if let Ok(handle) = service_control_handler::register(SERVICE_NAME, |_| ServiceControlHandlerResult::NoError) {
       let _ = handle.set_service_status(ServiceStatus {
           service_type: SERVICE_TYPE, current_state: ServiceState::Stopped,
           controls_accepted: ServiceControlAccept::empty(), exit_code: ServiceExitCode::ServiceSpecific(error_code),
           checkpoint: 0, wait_hint: Duration::default(), process_id: None,
       });
       warn!("Reported error {} to SCM and set status to Stopped.", error_code);
    } else { eprintln!("[{}] Failed to register handler to report error {} to SCM.", SERVICE_NAME, error_code); }
}

fn main() -> Result<()> {
    if let Err(e) = service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        eprintln!("[{}] Service dispatcher error: {}. Ensure the application is registered and run as a Windows service.", SERVICE_NAME, e);
        return Err(e);
    }
    Ok(())
}