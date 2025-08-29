use log::{trace};
use crate::language::token::{Token, TokenType};
use crate::language::error::{Result, Error, ErrorKind};
use crate::lexer_error;

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

    pub fn read_string(&mut self) -> Result<String> {
        self.consume();

        let mut string = String::new();

        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.consume();
                return Ok(string);
            } else if ch == '\\' {
                self.consume();

                if let Some(next_ch) = self.peek() {
                    match next_ch {
                        '"' => {
                            string.push('"');
                            self.consume();
                        },
                        '\\' => {
                            string.push('\\');
                            self.consume();
                        },
                        'n' => {
                            string.push('\n');
                            self.consume();
                        },
                        'r' => {
                            string.push('\r');
                            self.consume();
                        },
                        't' => {
                            string.push('\t');
                            self.consume();
                        },
                        _ => {
                            string.push(next_ch);
                            self.consume();
                        }
                    }
                } else {
                    return lexer_error!("Неожиданный конец файла после символа '\\' в строке", self.line, self.column);
                }
            } else {
                string.push(ch);
                self.consume();
            }
        }

        lexer_error!("Строка не закрыта", self.line, self.column)
    }

    pub fn read_comment(&mut self) -> Result<String> {
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
                            return lexer_error!("Многострочный комментарий не закрыт", self.line, self.column);
                        }
                    }
                    Ok(comment)
                },
                _ => lexer_error!("Неверный символ после '/'", self.line, self.column),
            }
        } else {
            lexer_error!("Неожиданный конец файла после '/'", self.line, self.column)
        }
    }

    fn read_number(&mut self) -> Result<i64> {
        let mut number = String::new();

        while let Some(ch) = self.peek() {
            if ch.is_digit(10) {
                number.push(ch);
                self.consume();
            } else {
                break;
            }
        }

        number.parse::<i64>().map_err(|_| Error {
            kind: ErrorKind::Lexer,
            message: format!("Не удалось преобразовать строку в число: {}", number),
            line: Some(self.line),
            column: Some(self.column),
        })
    }

    pub fn next_token(&mut self) -> Result<Token> {
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
                '[' => {
                  self.consume();
                    Ok(Token { token_type: TokenType::LBracket, line, column })
                },
                ']' => {
                  self.consume();
                    Ok(Token { token_type: TokenType::RBracket, line, column })
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
                    self.consume();
                    if self.peek() == None {
                        self.consume();
                        Ok(Token { token_type: TokenType::Divide, line, column })
                    } else if self.peek() == Some('=') {
                        self.consume();
                        Ok(Token { token_type: TokenType::DivideEqual, line, column })
                    } else {
                        match self.read_comment() {
                            Ok(comment) => Ok(Token { token_type: TokenType::Comment(comment), line, column }),
                            Err(e) => Err(e),
                        }
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
                        lexer_error!(format!("Неизвестный символ: '{}'", ch), line, column)
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
                        lexer_error!(format!("Неизвестный символ: '{}'", ch), line, column)
                    }
                },
                '&' => {
                    self.consume();
                    if self.peek() == Some('&') {
                        self.consume();
                        Ok(Token { token_type: TokenType::LogicalAnd, line, column })
                    } else {
                        lexer_error!(format!("Неизвестный символ: '&', ожидается '&&'"), line, column)
                    }
                },
                '|' => {
                    self.consume();
                    if self.peek() == Some('|') {
                        self.consume();
                        Ok(Token { token_type: TokenType::LogicalOr, line, column })
                    } else {
                        lexer_error!("Неизвестный символ: '|', ожидается '||'", line, column)
                    }
                },
                '*' => {
                    self.consume();
                    if self.peek() == Some('=') {
                        self.consume();
                        Ok(Token { token_type: TokenType::MultiplyEqual, line, column })
                    } else {
                        Ok(Token { token_type: TokenType::Multiply, line, column })
                    }
                },
                '-' => {
                  self.consume();
                    if self.peek() == Some('=') {
                        self.consume();
                        Ok(Token { token_type: TokenType::SubstructEqual, line, column })
                    } else {
                        Ok(Token { token_type: TokenType::Substruct, line, column })
                    }
                },
                '^' => {
                    self.consume();
                    if self.peek() == Some('=') {
                        self.consume();
                        Ok(Token { token_type: TokenType::PowerEqual, line, column })
                    } else {
                        Ok(Token { token_type: TokenType::Power, line, column })
                    }
                }
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
                        "for" => Ok(Token { token_type: TokenType::For, line, column }),
                        "while" => Ok(Token { token_type: TokenType::While, line, column }),
                        "in" => Ok(Token { token_type: TokenType::In, line, column }),
                        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" =>
                            Ok(Token { token_type: TokenType::HttpMethod(ident), line, column }),
                        _ => Ok(Token { token_type: TokenType::Identifier(ident), line, column }),
                    }
                },

                _ => lexer_error!(format!("Неизвестный символ: '{}'", ch), line, column),
            }
        } else {
            Ok(Token { token_type: TokenType::EOF, line, column })
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
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