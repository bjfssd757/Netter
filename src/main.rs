use clap::{Parser, Subcommand};
use commands::start::start_client;
use std::process::ExitCode;
use netter_logger;
use log::{
    info,
    error,
    trace,
    warn
};
use tokio::net::windows::named_pipe;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use netter_core::{
    Command,
    SuccessData,
    Response,
};

mod commands;
mod state;
mod core;

const PIPE_NAME: &str = r"\\.\pipe\MyNetterServicePipe";

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
    Ping,
    Start {
        #[arg(long = "kind", short = 'k', help="Тип сервера (tcp, udp, websocket, http)")]
        kind: String,

        #[arg(long = "path", help="Путь к конфигурационному файлу .rd")]
        path: Option<String>,

        #[arg(long = "host", help="IP адрес или хост для запуска сервера")]
        host: Option<String>,

        #[arg(long = "port", short = 'p', help="Порт для запуска сервера")]
        port: Option<u16>,

        #[arg(long, default_value_t = false, help="Защита сервера TLS (по умолчанию выключено)")]
        protect: bool,
    },
    Stop {
        #[arg(long = "id", short = 'i', help = "ID сервера для остановки сервера")]
        server_id: u32,
    },
    Parse {
        #[arg(long="path", short='p', help="Путь к конфигурационному файлу .rd")]
        path: String,
    },
    Client,
    Update,
    Auth {
        #[arg(short='u', long="user", help = "Имя пользователя для аутентификации")]
        user: String,
    },
    Shutdown {
        #[arg(help = "ID сервера для экстренного завершения", long="id", short='i')]
        server_id: u32,
    },
    Status {
        #[arg(help = "ID сервера для получения статуса (опционально)", long="id", short='i')]
        server_id: Option<u32>,
    },
    State {
        #[arg(help = "ID сервера для получения состояния", long="id", short='i')]
        server_id: u32,
    },
    List,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let log_dir = "logs";
    if let Err(e) = netter_logger::init(Some(log_dir), log::LevelFilter::Debug, log::LevelFilter::Debug) {
        eprintln!("CRITICAL: Не удалось инициализировать логгер: {}", e);
        return ExitCode::FAILURE;
    }

    trace!("Logger CLI initialized.");


    let command_to_send = match cli.subcommand {
        Some(Commands::Ping) => Command::Ping,
        Some(Commands::Start { kind, path: _, host, port, protect}) => {
            Command::CreateServer {
                kind,
                host: host.unwrap_or_else(|| "127.0.0.1".to_string()),
                port: port.unwrap_or(8080),
                protect,
            }
        },
        Some(Commands::Stop { server_id }) => Command::StopServer { server_id },
        Some(Commands::Shutdown { server_id }) => Command::ShutdownServer { server_id },
        Some(Commands::Status { server_id }) => Command::GetStatus { server_id },
        Some(Commands::State { server_id }) => Command::ServerState { id: server_id },
        Some(Commands::List) => Command::GetAllServers,
        Some(Commands::Auth { user }) => Command::Auth { user },
        Some(Commands::Update) => Command::Update,
        Some(Commands::Parse { path }) => {
            Command::Parse { path }
        },
        Some(Commands::Client) => {
            todo!();
        },
        None => {
            error!("Команда не указана. Используйте --help для справки.");
            return ExitCode::FAILURE;
        }
    };

    info!("Отправка команды службе: {:?}", command_to_send);
    match send_command_to_service(command_to_send).await {
        Ok(success_data) => {
            info!("Успех:");
            handle_success_response(success_data);
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!("Ошибка выполнения команды: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn handle_success_response(data: SuccessData) {
    match data {
        SuccessData::Pong => info!("  Pong! Служба доступна."),
        SuccessData::ServerStarted { task_id } => info!("  Сервер запущен с ID: {}", task_id),
        SuccessData::ServerStopped => info!("  Сервер остановлен."),
        SuccessData::ShutdownServer => info!("  Сервер экстренно завершен."),
        SuccessData::ServerCreated { task_id } => info!("  Сервер создан/запущен с ID: {}", task_id),
        SuccessData::StatusReport { status } => info!("  Статус: {}", status),
        SuccessData::StateReport { is_running, id, kind, host, port, protect, uptime, connections, logs } => {
            info!("  Детальное состояние сервера ID: {}", id);
            println!("    Статус: {}", if is_running { "Работает" } else { "Остановлен" });
            println!("    Тип: {}", kind);
            println!("    Адрес: {}:{}", host, port);
            println!("    Защита: {}", if protect { "Включена" } else { "Выключена" });
            println!("    Время работы: {} сек", uptime);
            println!("    Подключения: {}", connections);
            if !logs.is_empty() {
                println!("    Последние логи:");
                for log_entry in logs.iter().take(5) {
                    println!("      - {}", log_entry);
                }
            }
        },
        SuccessData::AllServersReport { servers } => {
            if servers.is_empty() {
                warn!("  Нет запущенных серверов.");
            } else {
                info!("  Список серверов:");
                for server in servers {
                    println!(
                        "    - ID: {}, Тип: {}, Адрес: {}:{}, Статус: Работает ({} сек)", // Предполагаем, что в списке только работающие
                        server.id, server.kind, server.host, server.port, server.uptime
                    );
                }
            }
        },
        SuccessData::AuthSuccess => info!("  Аутентификация прошла успешно."),
        SuccessData::UpdateSuccess => info!("  Обновление прошло успешно."),
        SuccessData::ParseSuccess => info!("  Парсинг прошел успешно."),
    }
}


async fn send_command_to_service(command: Command) -> Result<SuccessData, Box<dyn std::error::Error>> {
    trace!("Попытка подключения к каналу: {}", PIPE_NAME);

    let pipe_result = named_pipe::ClientOptions::new().open(PIPE_NAME);
    let mut pipe = match pipe_result {
        Ok(pipe) => pipe,
        Err(e) => {
            error!("Не удалось подключиться к службе: {}", e);
            let msg = format!(
                "Не удалось подключиться к службе по каналу '{}'. Убедитесь, что служба запущена.\n  Ошибка ОС: {}",
                PIPE_NAME, e
            );
            return Err(msg.into());
        }
    };
    trace!("Успешно подключено к каналу.");

    let serialized_command = match bincode::serialize(&command) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Ошибка сериализации команды: {}", e);
            return Err(format!("Внутренняя ошибка CLI: не удалось сериализовать команду ({})", e).into());
        }
    };
    trace!("Сериализованная команда ({} байт)", serialized_command.len());

    if let Err(e) = pipe.write_all(&serialized_command).await {
         error!("Ошибка отправки команды службе: {}", e);
         return Err(format!("Ошибка связи со службой при отправке команды ({})", e).into());
    }
    if let Err(e) = pipe.flush().await {
         error!("Ошибка flush после отправки команды: {}", e);
    }
    trace!("Команда отправлена службе.");

    let mut buffer = Vec::with_capacity(1024);
    match pipe.read_to_end(&mut buffer).await {
         Ok(0) => {
              warn!("Служба закрыла соединение, не отправив данных.");
              return Err("Служба не отправила ответ.".into());
         }
         Ok(n) => {
              trace!("Ответ получен ({} байт).", n);
         }
         Err(e) => {
              error!("Ошибка чтения ответа от службы: {}", e);
              return Err(format!("Ошибка связи со службой при чтении ответа ({})", e).into());
         }
    }

    let response: Response = match bincode::deserialize(&buffer) {
        Ok(resp) => resp,
        Err(e) => {
            error!("Ошибка десериализации ответа от службы: {}", e);
            let raw_response = String::from_utf8_lossy(&buffer);
            error!("Не удалось разобрать ответ: {}", raw_response);
            return Err(format!("Служба отправила некорректный ответ ({})", e).into());
        }
    };
    trace!("Десериализованный ответ: {:?}", response);

    match response {
        Response::Success(data) => Ok(data),
        Response::Error(core_error) => {
            warn!("Служба вернула ошибку: {}", core_error);
            Err(Box::new(core_error))
        }
    }
}