use std::fmt;
use crate::language::operators::{
    Token,
    TokenType,
};

pub struct Lexer {
    pub(crate) input: Vec<char>,
    pub(crate) position: usize,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn peek(&self) -> Option<char> {
        if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        }
    }

    pub fn peek_next(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }

    pub fn consume(&mut self) -> Option<char> {
        if self.position < self.input.len() {
            let ch = self.input[self.position];
            self.position += 1;
            
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            
            Some(ch)
        } else {
            None
        }
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.consume();
            } else {
                break;
            }
        }
    }

    pub fn read_identifier(&mut self) -> String {
        let mut identifier = String::new();
        
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '{' || ch == '}' {
                identifier.push(ch);
                self.consume();
            } else {
                break;
            }
        }
        
        identifier
    }

    pub fn read_string(&mut self) -> Result<String, String> {
        self.consume();
        
        let mut string = String::new();
        
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.consume();
                return Ok(string);
            } else {
                string.push(ch);
                self.consume();
            }
        }
        
        Err(format!("Строка не закрыта, строка {} колонка {}", self.line, self.column))
    }

    pub fn read_comment(&mut self) -> Result<String, String> {
        self.consume();
        
        let mut comment = String::new();
        
        if let Some(next_ch) = self.peek() {
            match next_ch {
                '/' => {
                    self.consume();
                    
                    while let Some(ch) = self.peek() {
                        if ch == '\n' {
                            break;
                        } else {
                            comment.push(ch);
                            self.consume();
                        }
                    }
                    Ok(comment)
                },
                '*' => {
                    self.consume();
                    
                    loop {
                        if self.peek() == Some('*') && self.peek_next() == Some('/') {
                            self.consume();
                            self.consume();
                            break;
                        } else if let Some(ch) = self.consume() {
                            comment.push(ch);
                        } else {
                            return Err(format!("Многострочный комментарий не закрыт, строка {}", self.line));
                        }
                    }
                    Ok(comment)
                },
                _ => Err(format!("Неверный символ после '/', строка {} колонка {}", self.line, self.column)),
            }
        } else {
            Err(format!("Неожиданный конец файла после '/', строка {} колонка {}", self.line, self.column))
        }
    }

    fn read_number(&mut self) -> Result<i64, String> {
        let mut number = String::new();
        
        while let Some(ch) = self.peek() {
            if ch.is_digit(10) {
                number.push(ch);
                self.consume();
            } else {
                break;
            }
        }
        
        number.parse::<i64>().map_err(|_| format!(
            "Не удалось преобразовать строку в число: {}, строка {} колонка {}", 
            number, self.line, self.column
        ))
    }

    pub fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace();
        
        let line = self.line;
        let column = self.column;
        
        if let Some(ch) = self.peek() {
            match ch {
                '{' => {
                    self.consume();
                    Ok(Token { token_type: TokenType::LBrace, line, column })
                },
                '}' => {
                    self.consume();
                    Ok(Token { token_type: TokenType::RBrace, line, column })
                },
                '(' => {
                    self.consume();
                    Ok(Token { token_type: TokenType::LParen, line, column })
                },
                ')' => {
                    self.consume();
                    Ok(Token { token_type: TokenType::RParen, line, column })
                },
                ';' => {
                    self.consume();
                    Ok(Token { token_type: TokenType::Semicolon, line, column })
                },
                '.' => {
                    self.consume();
                    Ok(Token { token_type: TokenType::Dot, line, column })
                },
                ',' => {
                    self.consume();
                    Ok(Token { token_type: TokenType::Comma, line, column })
                },
                '=' => {
                    self.consume();
                    if self.peek() == Some('=') {
                        self.consume();
                        Ok(Token { token_type: TokenType::DoubleEquals, line, column })
                    } else {
                        Ok(Token { token_type: TokenType::Equals, line, column })
                    }
                },
                '"' => {
                    match self.read_string() {
                        Ok(s) => Ok(Token { token_type: TokenType::String(s), line, column }),
                        Err(e) => Err(e),
                    }
                },
                '0'..='9' => {
                    let number = self.read_number()?;
                    Ok(Token { token_type: TokenType::Number(number), line, column })
                }
                '/' => {
                    match self.read_comment() {
                        Ok(comment) => Ok(Token { token_type: TokenType::Comment(comment), line, column }),
                        Err(e) => Err(e),
                    }
                },
                '?' => {
                    self.consume();
                    Ok(Token { token_type: TokenType::TryOperator, line, column })
                },
                '!' => {
                    self.consume();
                    if self.peek() == Some('!') {
                        self.consume();
                        Ok(Token { token_type: TokenType::UnwrapOperator, line, column })
                    } else if self.peek() == Some('=') {
                        self.consume();
                        Ok(Token { token_type: TokenType::NotEquals, line, column })
                    } else {
                        Err(format!("Неизвестный символ: '{}', строка {} колонка {}", ch, line, column))
                    }
                },
                '+' => {
                    self.consume();
                    if self.peek() == Some('=') {
                        self.consume();
                        Ok(Token { token_type: TokenType::PlusEqual, line, column })
                    } else {
                        Ok(Token { token_type: TokenType::Concatenation, line, column })
                    }
                },
                ':' => {
                    self.consume();
                    if self.peek() == Some(':') {
                        self.consume();
                        Ok(Token { token_type: TokenType::DoubleColon, line, column })
                    } else {
                        Err(format!("Неизвестный символ: '{}', строка {} колонка {}", ch, line, column))
                    }
                },
                _ if ch.is_alphabetic() => {
                    let ident = self.read_identifier();
                    match ident.as_str() {
                        "route" => Ok(Token { token_type: TokenType::Route, line, column }),
                        "val" => Ok(Token { token_type: TokenType::Val, line, column }),
                        "if" => Ok(Token { token_type: TokenType::If, line, column }),
                        "else" => Ok(Token { token_type: TokenType::Else, line, column }),
                        "tls" => Ok(Token { token_type: TokenType::Tls, line, column }),
                        "enabled" => Ok(Token { token_type: TokenType::Enabled, line, column }),
                        "cert_path" => Ok(Token { token_type: TokenType::CertPath, line, column }),
                        "key_path" => Ok(Token { token_type: TokenType::KeyPath, line, column }),
                        "global_error_handler" => Ok(Token { token_type: TokenType::GlobalErrorHandler, line, column }),
                        "onError" => Ok(Token { token_type: TokenType::OnError, line, column }),
                        "config" => Ok(Token { token_type: TokenType::Config, line, column }),
                        "type" => Ok(Token { token_type: TokenType::TypeName, line, column }),
                        "host" => Ok(Token { token_type: TokenType::Host, line, column }),
                        "port" => Ok(Token { token_type: TokenType::Port, line, column }),
                        "import" => Ok(Token { token_type: TokenType::Import, line, column }),
                        "as" => Ok(Token { token_type: TokenType::As, line, column }),
                        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" => 
                            Ok(Token { token_type: TokenType::HttpMethod(ident), line, column }),
                        _ => Ok(Token { token_type: TokenType::Identifier(ident), line, column }),
                    }
                },

                _ => Err(format!("Неизвестный символ: '{}', строка {} колонка {}", ch, line, column)),
            }
        } else {
            Ok(Token { token_type: TokenType::EOF, line, column })
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token()?;
            
            if token.token_type == TokenType::EOF {
                tokens.push(token);
                break;
            }
            
            if let TokenType::Comment(_) = token.token_type {
                continue;
            }
            
            tokens.push(token);
        }
        
        Ok(tokens)
    }
}

#[derive(Debug, Clone)]
pub enum AstNode {
    Program(Vec<Box<AstNode>>),
    Route {
        path: String,
        method: String,
        body: Box<AstNode>,
        on_error: Option<Box<AstNode>>,
    },
    Block(Vec<Box<AstNode>>),
    VarDeclaration {
        name: String,
        value: Box<AstNode>,
    },
    FunctionCall {
        object: Option<Box<AstNode>>,
        name: String,
        args: Vec<Box<AstNode>>,
        try_operator: bool,
        unwrap_operator: bool,
    },
    PropertyAccess {
        object: Box<AstNode>,
        property: String,
    },
    IfStatement {
        condition: Box<AstNode>,
        then_branch: Box<AstNode>,
        else_branch: Option<Box<AstNode>>,
    },
    StringLiteral(String),
    NumberLiteral(i64),
    Identifier(String),
    BinaryOp {
        left: Box<AstNode>,
        operator: String,
        right: Box<AstNode>,
    },
    TlsConfig {
        enabled: bool,
        cert_path: String,
        key_path: String,
    },
    ServerConfig {
        routes: Vec<Box<AstNode>>,
        tls_config: Option<Box<AstNode>>,
        global_error_handler: Option<Box<AstNode>>,
        config_block: Option<Box<AstNode>>,
    },
    GlobalErrorHandler {
        error_var: String,
        body: Box<AstNode>,
    },
    ErrorHandlerBlock {
        error_var: String,
        body: Box<AstNode>,
    },
    ConfigBlock {
        config_type: String,
        host: String,
        port: String,
    },
    Import {
        path: String,
        alias: String,
    }
}

impl fmt::Display for AstNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AstNode::Program(statements) => {
                writeln!(f, "Program:")?;
                for (i, stmt) in statements.iter().enumerate() {
                    write!(f, "  Маршрут {}: {}", i + 1, stmt)?;
                }
                Ok(())
            },
            AstNode::Route { path, method, body, on_error } => {
                match on_error {
                    Some(e) => {
                        writeln!(f, "Маршрут: {} {} {} {}", method, path, body, e)
                    },
                    None => {
                        writeln!(f, "Маршрут: {} {} {}", method, path, body)
                    }
                }
            },
            AstNode::Block(statements) => {
                writeln!(f, "{{")?;
                for stmt in statements {
                    write!(f, "    {}", stmt)?;
                }
                write!(f, "}}")
            },
            AstNode::VarDeclaration { name, value } => {
                writeln!(f, "val {} = {};", name, value)
            },
            AstNode::FunctionCall { object, name, args, try_operator, unwrap_operator } => {
                if let Some(obj) = object {
                    write!(f, "{}.{}(", obj, name)?;
                } else {
                    write!(f, "{}(", name)?;
                }
                let args_str: Vec<String> = args.iter().map(|arg| format!("{}", arg)).collect();
                match (try_operator, unwrap_operator) {
                    (true, false) => {
                        write!(f, "{})?", args_str.join(", "))
                    },
                    (false, true) => {
                        write!(f, "{})!!", args_str.join(", "))
                    },
                    (false, false) => {
                        write!(f, "{})", args_str.join(", "))
                    },
                    _ => {
                        write!(f, "{})!!?", args_str.join(", "))
                    }
                }
            },
            AstNode::PropertyAccess { object, property } => {
                write!(f, "{}.{}", object, property)
            },
            AstNode::IfStatement { condition, then_branch, else_branch} => {
                write!(f, "if ({}) {}", condition, then_branch)?;
                if let Some(else_stmt) = else_branch {
                    write!(f, " else {}", else_stmt)?;
                }
                writeln!(f)
            },
            AstNode::StringLiteral(value) => write!(f, "\"{}\"", value),
            AstNode::NumberLiteral(value) => write!(f, "{}", value),
            AstNode::Identifier(name) => write!(f, "{}", name),
            AstNode::BinaryOp { left, operator, right } => {
                write!(f, "{} {} {}", left, operator, right)
            },
            AstNode::TlsConfig { enabled, cert_path, key_path } => {
                writeln!(f, "TLS Configuration: {{")?;
                writeln!(f, "  enabled: {}", enabled)?;
                writeln!(f, "  cert_path: \"{}\"", cert_path)?;
                writeln!(f, "  key_path: \"{}\"", key_path)?;
                writeln!(f, "}}")
            },
            AstNode::ServerConfig { routes, tls_config, global_error_handler, config_block } => {
                writeln!(f, "Server Configuration: {{")?;
                if let Some(tls) = tls_config {
                    writeln!(f, "  {}", tls)?;
                }
                if let Some(handler) = global_error_handler {
                    writeln!(f, "  {}", handler)?;
                }
                if let Some(config) = config_block {
                    writeln!(f, "  {}", config)?;
                }
                for route in routes {
                    writeln!(f, "  {}", route)?;
                }
                writeln!(f, "}}")
            },
            AstNode::GlobalErrorHandler { error_var, body } => {
                writeln!(f, "Global Error Handler({}): {{", error_var)?;
                writeln!(f, "   {}", body)?;
                writeln!(f, "}}")
            },
            AstNode::ErrorHandlerBlock { error_var, body } => {
                writeln!(f, "Error Handler Block({}): {{", error_var)?;
                writeln!(f, "   {}", body)?;
                writeln!(f, "}}")
            },
            AstNode::ConfigBlock { config_type, host, port } => {
                writeln!(f, "Config: {{")?;
                writeln!(f, "   type: \"{}\"", config_type)?;
                writeln!(f, "   host: \"{}\"", host)?;
                writeln!(f, "   port: {}", port)?;
                writeln!(f, "}}")
            },
            AstNode::Import { path, alias } => {
                writeln!(f, "import \"{}\" as {}", path, alias)
            }
        }
    }
}