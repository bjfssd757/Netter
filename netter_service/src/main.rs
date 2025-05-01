use std::{ffi::OsString, sync::mpsc, time::Duration};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, Result,
};
use log::{error, info, trace, warn, LevelFilter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{PipeMode, ServerOptions};
use netter_core::{Command, Response};
use netter_logger;

const SERVICE_NAME: &str = "NetterService";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
const PIPE_NAME: &str = r"\\.\pipe\MyNetterServicePipe";
const SERVICE_LOG_DIR: &str = "C:\\ProgramData\\NetterService\\Logs";

define_windows_service!(ffi_service_main, service_main);

fn service_main(arguments: Vec<OsString>) {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    if let Err(e) = netter_logger::init(Some(SERVICE_LOG_DIR), LevelFilter::Debug, LevelFilter::Trace) {
        eprintln!("[{}] CRITICAL: Не удалось инициализировать логгер службы: {}", SERVICE_NAME, e);
        report_service_error_status(100);
        std::process::exit(100);
    }

    info!("Запуск службы NetterService (PID: {})...", std::process::id());
    if !arguments.is_empty() {
        info!("Аргументы запуска службы: {:?}", arguments);
    }

    match run_service(arguments, shutdown_tx, shutdown_rx) {
        Ok(_) => {
            info!("Служба NetterService успешно остановлена.");
        }
        Err(e) => {
            error!("Критическая ошибка службы NetterService: {}", e);
            report_service_error_status(1);
            std::process::exit(1);
        }
    }

    info!("Процесс службы NetterService завершен.");
}


fn run_service(
    _arguments: Vec<OsString>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: mpsc::Receiver<()>,
) -> Result<()> {

    info!("Инициализация логики службы NetterService...");

    let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                info!("Получен сигнал Stop/Shutdown от SCM.");
                shutdown_tx.send(()).unwrap_or_else(|e| {
                    warn!("Не удалось отправить сигнал остановки (возможно, служба уже останавливается): {}", e);
                });
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => {
                trace!("Получена не обрабатываемая команда SCM: {:?}", control_event);
                ServiceControlHandlerResult::NotImplemented
            }
        }
    })?;

    info!("Обработчик команд SCM зарегистрирован.");

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(5),
        process_id: None,
    })?;
    info!("Статус службы: StartPending");

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("netter-worker")
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            error!("Не удалось создать Tokio runtime: {}", e);
            status_handle.set_service_status(ServiceStatus {
                service_type: SERVICE_TYPE,
                current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::ServiceSpecific(2),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            })?;
            return Err(windows_service::Error::Winapi(e));
        }
    };
    info!("Tokio runtime создан.");

    let (error_tx, error_rx) = mpsc::channel::<Box<dyn std::error::Error + Send + Sync>>();
    let async_main_handle = rt.spawn(run_async_server(error_tx.clone()));
    info!("Асинхронный сервер IPC запущен в Tokio runtime.");

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;
    info!("Статус службы: Running. Ожидание подключений на '{}'", PIPE_NAME);

    let mut service_stop_reason = "Unknown".to_string();
    let mut final_exit_code = ServiceExitCode::Win32(0);

    loop {
        match error_rx.try_recv() {
            Ok(err) => {
                error!("Критическая ошибка в асинхронном сервере: {}", err);
                service_stop_reason = format!("Async server error: {}", err);
                final_exit_code = ServiceExitCode::ServiceSpecific(3);
                break;
            }
            Err(mpsc::TryRecvError::Empty) => {
                 match shutdown_rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(_) => {
                        info!("Получен сигнал остановки от SCM в основном цикле.");
                        service_stop_reason = "Signal from SCM".to_string();
                        final_exit_code = ServiceExitCode::Win32(0);
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if async_main_handle.is_finished() {
                             error!("Асинхронная задача сервера IPC завершилась неожиданно!");
                             service_stop_reason = "Async server task finished unexpectedly".to_string();
                             final_exit_code = ServiceExitCode::ServiceSpecific(4);
                             break;
                        }
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        error!("Канал сигнала остановки SCM закрыт!");
                        service_stop_reason = "SCM signal channel disconnected".to_string();
                        final_exit_code = ServiceExitCode::ServiceSpecific(5);
                        break;
                    }
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                 error!("Канал ошибок асинхронной части закрыт!");
                 if async_main_handle.is_finished() && service_stop_reason == "Unknown" {
                     warn!("Канал ошибок закрыт, но причина остановки неизвестна. Возможно, async задача завершилась штатно?");
                     service_stop_reason = "Async error channel disconnected, task finished".to_string();
                     final_exit_code = ServiceExitCode::Win32(0);
                 } else if service_stop_reason == "Unknown" {
                    service_stop_reason = "Async error channel disconnected unexpectedly".to_string();
                    final_exit_code = ServiceExitCode::ServiceSpecific(6);
                 }
                 break;
            }
        }
    }

    info!("Начинается процесс остановки службы (Причина: {})...", service_stop_reason);

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 1,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    })?;
    info!("Статус службы: StopPending");

    info!("Завершение работы Tokio runtime (ожидание до 5 секунд)...");
    rt.shutdown_timeout(Duration::from_secs(5));
    info!("Tokio runtime завершен.");

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: final_exit_code,
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;
    info!("Статус службы: Stopped (Код выхода: {:?})", final_exit_code);

    if matches!(final_exit_code, ServiceExitCode::Win32(0)) {
        Ok(())
    } else {
        Err(windows_service::Error::Winapi(std::io::Error::new(
            std::io::ErrorKind::Other,
            service_stop_reason,
        )))
    }
}


async fn run_async_server(error_tx: mpsc::Sender<Box<dyn std::error::Error + Send + Sync>>) {
    let mut server = match create_pipe_server() {
        Ok(s) => s,
        Err(e) => {
            let err_msg = format!("Критическая ошибка: Не удалось создать сервер именованных каналов '{}': {}", PIPE_NAME, e);
            error!("{}", err_msg);
            let _ = error_tx.send(Box::new(e));
            return;
        }
    };

    info!("Сервер IPC слушает на канале '{}'", PIPE_NAME);

    loop {
        match server.connect().await {
            Ok(_) => {
                trace!("Клиент подключился к каналу IPC.");
                let client_pipe = server;

                server = match create_pipe_server() {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Не удалось создать следующий экземпляр сервера канала '{}': {}", PIPE_NAME, e);
                        let _ = error_tx.send(Box::new(e));
                        break;
                    }
                };
                trace!("Создан новый экземпляр сервера IPC для ожидания следующего клиента.");

                tokio::spawn(async move {
                    handle_client(client_pipe).await;
                });
            }
            Err(e) => {
                error!("Ошибка ожидания подключения клиента к каналу '{}': {}. Попытка пересоздать сервер...", PIPE_NAME, e);
                tokio::time::sleep(Duration::from_secs(2)).await;
                server = match create_pipe_server() {
                    Ok(s) => s,
                    Err(e_retry) => {
                        let err_msg = format!("Критическая ошибка: Не удалось пересоздать сервер именованных каналов '{}' после ошибки: {}", PIPE_NAME, e_retry);
                        error!("{}", err_msg);
                        let _ = error_tx.send(Box::new(e_retry));
                        break;
                    }
                };
                warn!("Сервер именованных каналов '{}' пересоздан после ошибки.", PIPE_NAME);
            }
        }
    }
    warn!("Цикл асинхронного сервера IPC завершен.");
}

fn create_pipe_server() -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
     ServerOptions::new()
        .pipe_mode(PipeMode::Message)
        .first_pipe_instance(false)
        .reject_remote_clients(true)
        .create(PIPE_NAME)
}


async fn handle_client(mut pipe: tokio::net::windows::named_pipe::NamedPipeServer) {
    trace!("[Client] Начало обработки.");

    let mut buffer = Vec::with_capacity(1024);

    match pipe.read_to_end(&mut buffer).await {
        Ok(0) => {
            warn!("[Client] Отключился, не отправив команду.");
            return;
        }
        Ok(n) => {
            trace!("[Client] Прочитано {} байт команды.", n);

            match bincode::deserialize::<Command>(&buffer) {
                Ok(command) => {
                    info!("[Client] Получена команда: {:?}", command);

                    let response = netter_core::execute_command(command).await;

                    info!("[Client] Ответ на команду: {:?}", response);

                    match bincode::serialize(&response) {
                        Ok(response_bytes) => {
                            if let Err(e) = pipe.write_all(&response_bytes).await {
                                error!("[Client] Ошибка отправки ответа: {}", e);
                            } else {
                                trace!("[Client] Ответ успешно отправлен ({} байт).", response_bytes.len());
                            }
                            if let Err(e) = pipe.flush().await {
                                warn!("[Client] Ошибка flush при отправке ответа: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("[Client] Ошибка сериализации ответа: {}", e);
                            let error_response = Response::Error(netter_core::CoreError::InternalError(
                                format!("Failed to serialize service response: {}", e)
                            ));
                            if let Ok(err_bytes) = bincode::serialize(&error_response) {
                                let _ = pipe.write_all(&err_bytes).await;
                                let _ = pipe.flush().await;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("[Client] Ошибка десериализации команды: {}", e);
                     let raw_request = String::from_utf8_lossy(&buffer);
                     error!("[Client] Не удалось разобрать запрос: '{}'", raw_request);
                     let error_response = Response::Error(netter_core::CoreError::InvalidInput(
                        format!("Failed to deserialize command received by service: {}", e)
                    ));
                    if let Ok(err_bytes) = bincode::serialize(&error_response) {
                        let _ = pipe.write_all(&err_bytes).await;
                        let _ = pipe.flush().await;
                    }
                }
            }
        }
        Err(e) => {
            error!("[Client] Ошибка чтения команды из канала: {}", e);
        }
    }

    trace!("[Client] Завершение обработки.");
}

fn report_service_error_status(error_code: u32) {
    if let Ok(handle) = service_control_handler::register(SERVICE_NAME, |_| ServiceControlHandlerResult::NoError) {
        let _ = handle.set_service_status(ServiceStatus {
           service_type: SERVICE_TYPE,
           current_state: ServiceState::Stopped,
           controls_accepted: ServiceControlAccept::empty(),
           exit_code: ServiceExitCode::ServiceSpecific(error_code),
           checkpoint: 0,
           wait_hint: Duration::default(),
           process_id: None,
       });
    } else {
        eprintln!("[{}] Не удалось даже зарегистрировать обработчик для сообщения об ошибке SCM.", SERVICE_NAME);
    }
}

fn main() -> Result<()> {
    if let Err(e) = service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        eprintln!("[{}] Ошибка запуска диспетчера служб: {}", SERVICE_NAME, e);
        return Err(e);
    }
    Ok(())
}