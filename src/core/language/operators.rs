use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Route,              // route
    Val,                // val
    Var,                // var
    If,                 // if
    Else,               // else
    String(String),     // "..."
    Identifier(String), // Идентификаторы
    HttpMethod(String), // GET, POST ...
    LBrace,             // {
    RBrace,             // }
    LParen,             // (
    RParen,             // )
    Semicolon,          // ;
    Dot,                // .
    Comma,              // ,
    Equals,             // =
    DoubleEquals,       // ==
    Comment(String),    // Комментарии
    EOF,                // Конец файла
}

#[derive(Debug, Clone)]
pub struct Token {
    pub(crate) token_type: TokenType,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.token_type {
            TokenType::Route => write!(f, "route"),
            TokenType::Val => write!(f, "val"),
            TokenType::Var => write!(f, "var"),
            TokenType::If => write!(f, "if"),
            TokenType::Else => write!(f, "else"),
            TokenType::String(s) => write!(f, "\"{}\"", s),
            TokenType::Identifier(id) => write!(f, "{}", id),
            TokenType::HttpMethod(method) => write!(f, "{}", method),
            TokenType::LBrace => write!(f, "{{"),
            TokenType::RBrace => write!(f, "}}"),
            TokenType::LParen => write!(f, "("),
            TokenType::RParen => write!(f, ")"),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Dot => write!(f, "."),
            TokenType::Comma => write!(f, ","),
            TokenType::Equals => write!(f, "="),
            TokenType::DoubleEquals => write!(f, "=="),
            TokenType::Comment(c) => write!(f, "/* {} */", c),
            TokenType::EOF => write!(f, "EOF"),
        }
    }
}