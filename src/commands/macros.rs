use chrono::Local;
use colored::*;
use log::{Level, LevelFilter, trace};

const LOGS_DIRECTORY: &str = "Logs";
const LOGS_PREFIX: &str = "netter_log";
const SEPORATOR: &str = "_";
const FORMAT: &str = "%Y-%m-%d_%H-%M-%S";
const EXTENSION: &str = "log";

pub fn generate_name() -> String {
    let now = Local::now();
    let timestamp = now.format(FORMAT).to_string();
    
    format!(
        "{}/{}{}{}.{}",
        LOGS_DIRECTORY,
        LOGS_PREFIX,
        SEPORATOR,
        timestamp,
        EXTENSION
    )
}

fn ensure_log_directory_exists() -> std::io::Result<()> {
    let log_dir = std::path::Path::new(LOGS_DIRECTORY);
    if !log_dir.exists() {
        trace!("Директория логов '{}' не существует, создаём...", LOGS_DIRECTORY);
        std::fs::create_dir_all(log_dir)?;
        trace!("Директория логов успешно создана");
    }
    Ok(())
}

pub fn init(log_file: Option<&str>) -> Result<(), fern::InitError> {
    if let Some(_) = log_file.clone() {
        ensure_log_directory_exists().map_err(|e| {
            fern::InitError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Не удалось создать директорию для логов: {}", e),
            ))
        })?;
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
            
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            
            out.finish(format_args!(
                "[{}] [{}] [{}:{}] {}",
                timestamp,
                level_str,
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                message
            ))
        })
        .chain(std::io::stdout());

    let mut builder = fern::Dispatch::new()
        .format(|out, message, record| {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            
            out.finish(format_args!(
                "[{}] [{}] [{}:{}] {}",
                timestamp,
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                message
            ))
        })
        .level(LevelFilter::Trace)
        .chain(console_dispatch);

    if let Some(log_path) = log_file {
        builder = builder.chain(fern::log_file(log_path)?);
    }

    builder.apply()?;

    Ok(())
}

#[macro_export]
macro_rules! log_custom {
    ($level:expr, $($arg:tt)+) => {{
        let level = $level;
        let msg = format!($($arg)+);
        
        match level {
            log::Level::Error => log::error!("{}", msg),
            log::Level::Warn => log::warn!("{}", msg),
            log::Level::Info => log::info!("{}", msg),
            log::Level::Debug => log::debug!("{}", msg),
            log::Level::Trace => log::trace!("{}", msg),
        }
    }};
}

#[macro_export]
macro_rules! error_log {
    ($($arg:tt)+) => { $crate::log_custom!(log::Level::Error, $($arg)+) };
}

#[macro_export]
macro_rules! warn_log {
    ($($arg:tt)+) => { $crate::log_custom!(log::Level::Warn, $($arg)+) };
}

#[macro_export]
macro_rules! info_log {
    ($($arg:tt)+) => { $crate::log_custom!(log::Level::Info, $($arg)+) };
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)+) => { $crate::log_custom!(log::Level::Debug, $($arg)+) };
}

#[macro_export]
macro_rules! trace_log {
    ($($arg:tt)+) => { $crate::log_custom!(log::Level::Trace, $($arg)+) };
}