use std::fs;
use std::path::Path;
use log::trace;
use crate::language::error::{Result, Error, ErrorKind};

pub struct FileSystem {}

impl FileSystem {
    pub fn exists(path: &str) -> Result<bool> {
        trace!("Проверка существования файла: {}", path);
        Ok(Path::new(path).exists())
    }

    pub fn read_text(path: &str) -> Result<String> {
        trace!("Чтение текстового файла: {}", path);
        fs::read_to_string(path).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка чтения файла {}: {}", path, e),
            line: None,
            column: None,
        })
    }

    pub fn write_text(path: &str, content: &str) -> Result<()> {
        trace!("Запись в текстовый файл: {}", path);
        fs::write(path, content).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка записи в файл {}: {}", path, e),
            line: None,
            column: None,
        })
    }

    pub fn is_directory(path: &str) -> Result<bool> {
        trace!("Проверка директории: {}", path);
        Ok(Path::new(path).is_dir())
    }

    pub fn list_files(dir_path: &str) -> Result<String> {
        trace!("Получение списка файлов: {}", dir_path);

        let path = Path::new(dir_path);
        if !path.is_dir() {
            return Err(Error {
                kind: ErrorKind::Runtime,
                message: format!("Путь не является директорией: {}", dir_path),
                line: None,
                column: None,
            });
        }

        let entries = fs::read_dir(path).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка чтения директории {}: {}", dir_path, e),
            line: None,
            column: None,
        })?;

        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| Error {
                kind: ErrorKind::Runtime,
                message: format!("Ошибка доступа к файлу в директории {}: {}", dir_path, e),
                line: None,
                column: None,
            })?;

            if let Some(file_name) = entry.file_name().to_str() {
                files.push(file_name.to_string());
            }
        }

        serde_json::to_string(&files).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка сериализации списка файлов: {}", e),
            line: None,
            column: None,
        })
    }
}