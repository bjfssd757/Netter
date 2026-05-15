use std::fs;
use std::path::Path;
use log::trace;
use crate::language::error::{Result, Error, ErrorKind};
use crate::language::interpreter::Object;
use crate::language::rdl_types::RDLTypes;
use crate::runtime_error;

pub struct FileSystem {}

impl Object for FileSystem {
    fn name(&self) -> &'static str {
        "FileSystem"
    }

    fn methods(&self) -> Vec<&str> {
        vec!["exists", "read_text", "write_text", "is_directory", "list_files"]
    }

    fn call_method(&mut self, name: &str, args: Vec<RDLTypes>) -> Result<RDLTypes> {
        match name {
            "exists" => {
                if args.len() < 1 {
                    return runtime_error!(format!("Method FileSystem.exists required 1 argument"));
                }

                FileSystem::exists(&args[0]).map(|v| v.into())
            }
            "read_text" => {
                if args.len() < 1 {
                    return runtime_error!(format!("Method FileSystem.read_text required 1 argument"));
                }

                FileSystem::read_text(&args[0])
            }
            "write_text" => {
                if args.len() < 2 {
                    return runtime_error!(format!("Method FileSystem.write_text required 2 argument"));
                }

                FileSystem::write_text(&args[0], &args[1])?;
                Ok(RDLTypes::Boolean(true))
            }
            "is_directory" => {
                if args.len() < 1 {
                    return runtime_error!(format!("Method FileSystem.is_directory required 1 argument"));
                }

                FileSystem::is_directory(&args[0]).map(|v| v.into())
            }
            "list_files" => {
                if args.len() < 1 {
                    return runtime_error!(format!("Method FileSystem.list_files required 1 argument"));
                }

                FileSystem::list_files(&args[0])
            },
            _ => runtime_error!(format!("Function with name '{}' not found in FileSystem object", name))
        }
    }

    fn get_property(&self, _name: &str) -> RDLTypes {
        RDLTypes::Boolean(false)
    }

    fn method_exist(&self, name: &str) -> bool {
        self.methods().contains(&name)
    }

    fn properties(&self) -> Vec<&str> {
        vec![]
    }

    fn property_exist(&self, _name: &str) -> bool {
        false
    }
}

impl FileSystem {
    pub fn exists(path: &RDLTypes) -> Result<bool> {
        trace!("Проверка существования файла: {}", path);
        Ok(Path::new(path.to_string().as_str()).exists())
    }

    pub fn read_text(path: &RDLTypes) -> Result<RDLTypes> {
        trace!("Чтение текстового файла: {}", path);
        Ok(RDLTypes::String(fs::read_to_string(path.to_string()).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка чтения файла {}: {}", path, e),
            line: None,
            column: None,
        })?))
    }

    pub fn write_text(path: &RDLTypes, content: &RDLTypes) -> Result<()> {
        trace!("Запись в текстовый файл: {}", path);
        fs::write(path.to_string(), content.to_string()).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка записи в файл {}: {}", path, e),
            line: None,
            column: None,
        })
    }

    pub fn is_directory(path: &RDLTypes) -> Result<bool> {
        trace!("Проверка директории: {}", path);
        Ok(Path::new(path.to_string().as_str()).is_dir())
    }

    pub fn list_files(dir_path: &RDLTypes) -> Result<RDLTypes> {
        trace!("Получение списка файлов: {}", dir_path);
        
        let dir = dir_path.to_string();
        let path = Path::new(dir.as_str());
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

        Ok(RDLTypes::String(serde_json::to_string(&files).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка сериализации списка файлов: {}", e),
            line: None,
            column: None,
        })?))
    }
}