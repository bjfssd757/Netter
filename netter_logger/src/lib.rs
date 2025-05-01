use chrono::Local;
use colored::*;
use log::{Level, LevelFilter};
use std::path::{Path, PathBuf};
use std::io::ErrorKind;

const LOGS_PREFIX: &str = "netter_log";
const SEPARATOR: &str = "_";
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";
const LOG_EXTENSION: &str = "log";
const CONSOLE_TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

fn generate_filename_only() -> String {
    let now = Local::now();
    let timestamp = now.format(TIMESTAMP_FORMAT).to_string();
    format!("{}{}{}.{}", LOGS_PREFIX, SEPARATOR, timestamp, LOG_EXTENSION)
}

fn ensure_log_directory_exists(log_dir: &Path) -> std::io::Result<()> {
    if !log_dir.exists() {
        std::fs::create_dir_all(log_dir)?;
    }
    Ok(())
}

pub fn init(
    log_dir: Option<impl AsRef<Path>>,
    console_level: LevelFilter,
    file_level: LevelFilter,
) -> Result<(), fern::InitError> {

    let mut log_file_path: Option<PathBuf> = None;

    if let Some(dir) = log_dir {
        let dir_path = dir.as_ref();
        if let Err(e) = ensure_log_directory_exists(dir_path) {
             eprintln!("CRITICAL: Не удалось создать директорию для логов '{}': {}", dir_path.display(), e);
             return Err(fern::InitError::Io(std::io::Error::new(
                ErrorKind::Other,
                format!("Не удалось создать директорию для логов '{}': {}", dir_path.display(), e),
            )));
        }
        log_file_path = Some(dir_path.join(generate_filename_only()));
    }

    let console_dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            let level_str = match record.level() {
                Level::Error => "ERROR".red().bold(),
                Level::Warn => "WARN ".yellow().bold(),
                Level::Info => "INFO ".green().bold(),
                Level::Debug => "DEBUG".blue().bold(),
                Level::Trace => "TRACE".magenta().bold(),
            };

            let timestamp = Local::now().format(CONSOLE_TIMESTAMP_FORMAT).to_string();

            let thread_name = std::thread::current().name().unwrap_or("unnamed").to_string();
            let target = record.target();

            out.finish(format_args!(
                "[{}] [{}] [{}] [{}] {}",
                timestamp,
                level_str,
                thread_name,
                target,
                message
            ))
        })
        .level(console_level)
        .chain(std::io::stdout());

    let mut base_dispatch = fern::Dispatch::new()
        .level(LevelFilter::Trace)
        .chain(console_dispatch);

    if let Some(path) = &log_file_path {
        let file_dispatch = fern::Dispatch::new()
            .format(|out, message, record| {
                let timestamp = Local::now().format(CONSOLE_TIMESTAMP_FORMAT).to_string();
                 let thread_name = std::thread::current().name().unwrap_or("unnamed").to_string();
                 let target = record.target();
                out.finish(format_args!(
                    "[{}] [{:<5}] [{}] [{}] [{}:{}] {}",
                    timestamp,
                    record.level(),
                    thread_name,
                    target,
                    record.file().unwrap_or("?"),
                    record.line().unwrap_or(0),
                    message
                ))
            })
            .level(file_level)
            .chain(fern::log_file(path)?);

        base_dispatch = base_dispatch.chain(file_dispatch);
    }

    base_dispatch.apply()?;

    log::info!("Логгер инициализирован. Уровень консоли: {}, Уровень файла: {}", console_level, file_level);
    if let Some(path) = &log_file_path {
         log::info!("Запись логов в файл: {}", path.display());
    } else {
         log::info!("Запись логов в файл отключена.");
    }

    Ok(())
}