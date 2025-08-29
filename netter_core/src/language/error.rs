use std::fmt;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Lexer,
    Parser,
    Interpreter,
    Runtime,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.line, &self.column) {
            (Some(line), Some(column)) => {
                write!(f, "{:?} ошибка: {} (строка {}, колонка {})", self.kind, self.message, line, column)
            }
            (Some(line), None) => {
                write!(f, "{:?} ошибка: {} (строка {})", self.kind, self.message, line)
            }
            _ => {
                write!(f, "{:?} ошибка: {}", self.kind, self.message)
            }
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[macro_export]
macro_rules! lexer_error {
    ($msg:expr, $line:expr, $col:expr) => {
        Err($crate::language::error::Error {
            kind: $crate::language::error::ErrorKind::Lexer,
            message: $msg.to_string(),
            line: Some($line),
            column: Some($col),
        })
    };
}

#[macro_export]
macro_rules! parser_error {
    ($msg:expr, $line:expr, $col:expr) => {
        Err($crate::language::error::Error {
            kind: $crate::language::error::ErrorKind::Parser,
            message: $msg.to_string(),
            line: Some($line),
            column: Some($col),
        })
    };
}

#[macro_export]
macro_rules! interpreter_error {
    ($msg:expr) => {
        Err($crate::language::error::Error {
            kind: $crate::language::error::ErrorKind::Interpreter,
            message: $msg.to_string(),
            line: None,
            column: None,
        })
    };
}

#[macro_export]
macro_rules! runtime_error {
    ($msg:expr) => {
        Err($crate::language::error::Error {
            kind: $crate::language::error::ErrorKind::Runtime,
            message: $msg.to_string(),
            line: None,
            column: None,
        })
    };
}