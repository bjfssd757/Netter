use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Route,              // route
    Val,                // val
    Var,                // var
    If,                 // if
    Else,               // else
    // -------- //
    String(String),     // "..."
    Number(i64),        // 1234...
    // -------- //
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
    NotEquals,          // !=
    Comment(String),    // Комментарии
    EOF,                // Конец файла
    // -------- //
    Tls,                // tls
    Enabled,            // enabled
    CertPath,           // cert_path
    KeyPath,            // key_path
    // -------- //
    TryOperator,        // ?
    UnwrapOperator,     // !!
    OnError,            // on_error
    GlobalErrorHandler, // global_error_handler
    // -------- //
    Config,             // config
    TypeName,           // type
    Host,               // host
    Port,               // port
    // -------- //
    Concatenation,      // +
    PlusEqual,          // +=
    // -------- //
    Import,            // import
    As,                // as
    DoubleColon,       // ::
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
            TokenType::Number(n) => write!(f, "{}", n),
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
            TokenType::NotEquals => write!(f, "!="),
            TokenType::Comment(c) => write!(f, "/* {} */", c),
            TokenType::EOF => write!(f, "EOF"),
            TokenType::Tls => write!(f, "tls"),
            TokenType::Enabled => write!(f, "enabled"),
            TokenType::CertPath => write!(f, "cert_path"),
            TokenType::KeyPath => write!(f, "key_path"),
            TokenType::TryOperator => write!(f, "?"),
            TokenType::UnwrapOperator => write!(f, "!!"),
            TokenType::OnError => write!(f, "onError"),
            TokenType::GlobalErrorHandler => write!(f, "global_error_handler"),
            TokenType::Config => write!(f, "config"),
            TokenType::TypeName => write!(f, "type"),
            TokenType::Host => write!(f, "host"),
            TokenType::Port => write!(f, "port"),
            TokenType::Concatenation => write!(f, "+"),
            TokenType::PlusEqual => write!(f, "+="),
            TokenType::Import => write!(f, "import"),
            TokenType::As => write!(f, "as"),
            TokenType::DoubleColon => write!(f, "::"),
        }
    }
}