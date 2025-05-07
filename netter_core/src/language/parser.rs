use std::f64::consts::E;

use crate::language::operators::{
    Token,
    TokenType,
};
use crate::language::lexer::{
    AstNode,
    Lexer,
};
use log::{
    info,
    error,
    debug
};


pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
        }
    }

    pub fn parse(&mut self) -> Result<AstNode, String> {
        let mut statements = Vec::new();
        let mut tls_config = None;
        let mut global_error_handler = None;
        let mut config = None;
        let mut imports = Vec::new();
        
        while !self.is_at_end() {
            if self.check(&TokenType::Tls) {
                if tls_config.is_some() {
                    return Err("Дублирование TLS конфигурации".to_string());
                }
                tls_config = Some(Box::new(self.tls_config()?));
            } else if self.check(&TokenType::GlobalErrorHandler) {
                if global_error_handler.is_some() {
                    return Err("Дублирование глобального обработчика ошибок".to_string());
                }
                global_error_handler = Some(Box::new(self.global_error_handler()?));
            } else if self.check(&TokenType::Route) {
                statements.push(Box::new(self.route()?));
            } else if self.check(&TokenType::Config) {
                if config.is_some() {
                    return Err("Дублирование блока 'config'".to_string());
                }
                config = Some(Box::new(self.config_block()?));
            } else if self.check(&TokenType::Import) {
                imports.push(Box::new(self.import()?));
            } else {
                return Err(format!("Ожидается 'route', 'tls' или 'config', получено: {:?}", self.peek().token_type));
            }
        }

        let has_tls = tls_config.is_some();
        let has_global_handler = global_error_handler.is_some();
        let has_config = config.is_some();
        let program_statements = imports.into_iter().chain(statements.into_iter()).collect();
        
        if has_tls || has_global_handler || has_config {
            Ok(AstNode::ServerConfig {
                routes: program_statements,
                tls_config,
                global_error_handler,
                config_block: config,
            })
        } else if !program_statements.is_empty() {
            Ok(AstNode::Program(program_statements))
        } else {
            Ok(AstNode::Program(vec![]))
        }
    }

    fn import(&mut self) -> Result<AstNode, String> {
        self.consume(&TokenType::Import, "Ожидается ключевое слово 'import'")?;
    
        let path_token = self.consume(&TokenType::String(String::new()), "Ожидается строка пути к плагину")?;
        let path = match &path_token.token_type {
            TokenType::String(s) => s.clone(),
            _ => return Err("Ошибка парсинга пути к плагину".to_string()),
        };
    
        self.consume(&TokenType::As, "Ожидается ключевое слово 'as' после пути")?;
    
        let alias_token = self.consume(&TokenType::Identifier(String::new()), "Ожидается псевдоним плагина")?;
        let alias = match &alias_token.token_type {
            TokenType::Identifier(n) => n.clone(),
            _ => return Err("Ошибка парсинга псевдонима плагина".to_string()),
        };
    
        self.consume(&TokenType::Semicolon, "Ожидается ';' после импорта плагина")?;
    
        Ok(AstNode::Import { path, alias })
    }

    fn global_error_handler(&mut self) -> Result<AstNode, String> {
        self.consume(&TokenType::GlobalErrorHandler, "Ожидается ключевое слово 'global_error_handler'")?;
        self.consume(&TokenType::LParen, "Ожидается '(' после 'global_error_handler'")?;

        let error_var_token = self.consume(&TokenType::Identifier(String::new()), "Ожидается имя переменной")?;
        let error_var = match &error_var_token.token_type {
            TokenType::Identifier(name) => name.clone(),
            _ => return Err("Невозможно получить имя переменной".to_string()),
        };

        self.consume(&TokenType::RParen, "Ожидается ')' после имени переменной")?;

        let body = self.block()?;

        self.consume(&TokenType::Semicolon, "Ожидается ';' после блока глобального обработчика ошибок")?;

        Ok(AstNode::GlobalErrorHandler {
            error_var,
            body: Box::new(body),
        })
    }

    fn tls_config(&mut self) -> Result<AstNode, String> {
        self.consume(&TokenType::Tls, "Ожидается ключевое слово 'tls'")?;
        self.consume(&TokenType::LBrace, "Ожидается '{' после 'tls'")?;
        
        let mut enabled = false;
        let mut cert_path = String::new();
        let mut key_path = String::new();
        
        while !self.check(&TokenType::RBrace) && !self.is_at_end() {
            if self.match_token(&TokenType::Enabled) {
                self.consume(&TokenType::Equals, "Ожидается '=' после 'enabled'")?;
                if self.match_token(&TokenType::Identifier(String::new())) {
                    let value = match &self.previous().token_type {
                        TokenType::Identifier(v) => v.clone(),
                        _ => return Err("Ожидается bool значение для enabled".to_string()),
                    };
                    enabled = value == "true";
                } else {
                    return Err("Ожидается булево значение (true/false) для enabled".to_string());
                }
                self.consume(&TokenType::Semicolon, "Ожидается ';' после значения")?;
            } else if self.match_token(&TokenType::CertPath) {
                self.consume(&TokenType::Equals, "Ожидается '=' после 'cert_path'")?;
                if self.match_token(&TokenType::String(String::new())) {
                    cert_path = match &self.previous().token_type {
                        TokenType::String(v) => v.clone(),
                        _ => return Err("Ожидается строка для cert_path".to_string()),
                    };
                } else {
                    return Err("Ожидается строковое значение для cert_path".to_string());
                }
                self.consume(&TokenType::Semicolon, "Ожидается ';' после значения")?;
            } else if self.match_token(&TokenType::KeyPath) {
                self.consume(&TokenType::Equals, "Ожидается '=' после 'key_path'")?;
                if self.match_token(&TokenType::String(String::new())) {
                    key_path = match &self.previous().token_type {
                        TokenType::String(v) => v.clone(),
                        _ => return Err("Ожидается строка для key_path".to_string()),
                    };
                } else {
                    return Err("Ожидается строковое значение для key_path".to_string());
                }
                self.consume(&TokenType::Semicolon, "Ожидается ';' после значения")?;
            } else {
                return Err(format!("Неизвестный ключ в TLS конфигурации: {:?}", self.peek().token_type));
            }
        }
        
        self.consume(&TokenType::RBrace, "Ожидается '}' после TLS конфигурации")?;
        self.consume(&TokenType::Semicolon, "Ожидается ';' после блока TLS конфигурации")?;
        
        Ok(AstNode::TlsConfig {
            enabled,
            cert_path,
            key_path,
        })
    }

    fn config_block(&mut self) -> Result<AstNode, String> {
        self.consume(&TokenType::Config, "Ожидается ключевое слово 'config'")?;
        self.consume(&TokenType::LBrace, "Ожидается '{' после 'config'")?;

        let mut type_name = String::new();
        let mut host = String::new();
        let mut port = String::new();

        while !self.check(&TokenType::RBrace) && !self.is_at_end() {
            if self.match_token(&TokenType::TypeName) {
                self.consume(&TokenType::Equals, "Ожидается '=' после 'type'")?;
                let value_token = self.advance();
                 match &value_token.token_type {
                     TokenType::String(v) | TokenType::Identifier(v) => {
                        type_name = v.clone();
                    }
                    _ => return Err(format!("Ожидается строка или идентификатор для type, получено {:?}", value_token.token_type)),
                };
                self.consume(&TokenType::Semicolon, "Ожидается ';' после значения type")?;
            } else if self.match_token(&TokenType::Host) {
                self.consume(&TokenType::Equals, "Ожидается '=' после 'host'")?;
                let value_token = self.advance();
                 match &value_token.token_type {
                     TokenType::String(v) | TokenType::Identifier(v) => {
                        host = v.clone();
                    }
                    _ => return Err(format!("Ожидается строка или идентификатор для host, получено {:?}", value_token.token_type)),
                };
                self.consume(&TokenType::Semicolon, "Ожидается ';' после значения host")?;
            } else if self.match_token(&TokenType::Port) {
                self.consume(&TokenType::Equals, "Ожидается '=' после 'port'")?;
                let value_token = self.advance();
                 match &value_token.token_type {
                    TokenType::String(v) => port = v.clone(),
                    TokenType::Number(n) => port = n.to_string(),
                    TokenType::Identifier(v) => port = v.clone(),
                    _ => return Err(format!("Ожидается строка, число или идентификатор для port, получено {:?}", value_token.token_type)),
                };
                self.consume(&TokenType::Semicolon, "Ожидается ';' после значения port")?;
            } else {
                return Err(format!("Неизвестный ключ в блоке 'config': {:?} на строке {}", self.peek().token_type, self.peek().line));
            }
        }

        self.consume(&TokenType::RBrace, "Ожидается '}' после блока 'config'")?;
        self.consume(&TokenType::Semicolon, "Ожидается ';' после блока 'config'")?;

        if type_name == "http" && (host.is_empty() || port.is_empty()) {
             return Err("Для type=\"http\" необходимо указать host и port в блоке 'config'".to_string());
        }
        if !port.is_empty() && port.parse::<u16>().is_err() {
            return Err(format!("Значение port '{}' не является допустимым числом (0-65535)", port));
        }


        Ok(AstNode::ConfigBlock {
            config_type: type_name,
            host,
            port,
        })
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::EOF)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            match (&self.peek().token_type, token_type) {
                (TokenType::Identifier(_), TokenType::Identifier(_)) => true,
                (TokenType::String(_), TokenType::String(_)) => true,
                (TokenType::HttpMethod(_), TokenType::HttpMethod(_)) => true,
                (TokenType::Comment(_), TokenType::Comment(_)) => true,
                (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
            }
        }
    }

    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, token_type: &TokenType, error_message: &str) -> Result<&Token, String> {
        if self.check(token_type) {
            Ok(self.advance())
        } else {
            Err(format!("{}, получено: {:?}, строка: {}, колонка: {}", 
                error_message, self.peek().token_type, self.peek().line, self.peek().column))
        }
    }

    fn route(&mut self) -> Result<AstNode, String> {
        self.consume(&TokenType::Route, "Ожидается ключевое слово 'route'")?;

        let path_token = self.consume(&TokenType::String(String::new()), "Ожидается строка пути маршрута")?;
        let path = match &path_token.token_type {
            TokenType::String(s) => s.clone(),
            _ => return Err("Невозможный случай при парсинге пути маршрута".to_string()),
        };

        let method_token = self.consume(&TokenType::HttpMethod(String::new()), "Ожидается HTTP метод")?;
        let method = match &method_token.token_type {
            TokenType::HttpMethod(m) => m.clone(),
            _ => return Err("Невозможный случай при парсинге HTTP метода".to_string()),
        };

        let body = self.block()?;

        let on_error = if self.match_token(&TokenType::OnError) {
            Some(Box::new(self.error_handler()?))
        } else {
            None
        };

        self.consume(&TokenType::Semicolon, "Ожидается ';' после блока маршрута")?;

        Ok(AstNode::Route {
            path,
            method,
            body: Box::new(body),
            on_error,
        })
    }

    fn error_handler(&mut self) -> Result<AstNode, String> {
        self.consume(&TokenType::LParen, "Ожидается '(' после 'on_error'")?;

        let error_var_token = self.consume(&TokenType::Identifier(String::new()), "Ожидается имя переменной")?;
        let error_var = match &error_var_token.token_type {
            TokenType::Identifier(name) => name.clone(),
            _ => return Err("Невозможно получить имя переменной ошибки".to_string()),
        };

        self.consume(&TokenType::RParen, "Ожидается ')' после имени переменной ошибки")?;

        let body = self.block()?;

        Ok(AstNode::ErrorHandlerBlock {
            error_var,
            body: Box::new(body),
        })
    }

    fn block(&mut self) -> Result<AstNode, String> {
        if !self.check(&TokenType::LBrace) {
            return Err(format!(
                "Ожидается '{{', получено: {:?}, строка: {}, колонка: {}", 
                self.peek().token_type, self.peek().line, self.peek().column
            ));
        }
        
        self.consume(&TokenType::LBrace, "Ожидается '{'")?;
    
        let mut statements = Vec::new();
        
        if self.check(&TokenType::RBrace) {
            self.advance();
            return Ok(AstNode::Block(statements));
        }
        
        while !self.check(&TokenType::RBrace) && !self.is_at_end() {
            statements.push(Box::new(self.statement()?));
        }
    
        if !self.check(&TokenType::RBrace) {
            return Err(format!(
                "Ожидается '}}' после блока кода, получено: {:?}, строка: {}, колонка: {}", 
                self.peek().token_type, self.peek().line, self.peek().column
            ));
        }
        
        self.consume(&TokenType::RBrace, "Ожидается '}' после блока")?;
    
        Ok(AstNode::Block(statements))
    }

    fn statement(&mut self) -> Result<AstNode, String> {
        if self.match_token(&TokenType::Val) || self.match_token(&TokenType::Var) {
            self.var_declaration()
        } else if self.match_token(&TokenType::If) {
            self.if_statement()
        } else {
            self.expression_statement()
        }
    }

    fn var_declaration(&mut self) -> Result<AstNode, String> {
        let name_token = self.consume(&TokenType::Identifier(String::new()), "Ожидается имя переменной")?;
        let name = match &name_token.token_type {
            TokenType::Identifier(n) => n.clone(),
            _ => return Err("Невозможный случай при парсинге имени переменной".to_string()),
        };

        self.consume(&TokenType::Equals, "Ожидается '=' после имени переменной")?;

        let value = self.expression()?;

        self.consume(&TokenType::Semicolon, "Ожидается ';' после объявления переменной")?;

        Ok(AstNode::VarDeclaration {
            name,
            value: Box::new(value),
        })
    }

    fn if_statement(&mut self) -> Result<AstNode, String> {
        self.consume(&TokenType::LParen, "Ожидается '(' после 'if'")?;
        let condition = self.expression()?;
        self.consume(&TokenType::RParen, "Ожидается ')' после условия")?;
    
        let then_branch = self.block()?;
    
        let else_branch = if self.match_token(&TokenType::Else) {
            if self.match_token(&TokenType::If) {
                let inner_if = self.if_statement_no_semicolon()?;
                Some(Box::new(inner_if))
            } else {
                Some(Box::new(self.block()?))
            }
        } else {
            None
        };
    
        if else_branch.is_none() {
            self.consume(&TokenType::Semicolon, "Ожидается ';' после оператора if")?;
        }
    
        Ok(AstNode::IfStatement {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
        })
    }
    
    fn if_statement_no_semicolon(&mut self) -> Result<AstNode, String> {
        self.consume(&TokenType::LParen, "Ожидается '(' после 'if'")?;
        let condition = self.expression()?;
        self.consume(&TokenType::RParen, "Ожидается ')' после условия")?;
    
        let then_branch = self.block()?;
    
        let else_branch = if self.match_token(&TokenType::Else) {
            if self.match_token(&TokenType::If) {
                Some(Box::new(self.if_statement_no_semicolon()?))
            } else {
                Some(Box::new(self.block()?))
            }
        } else {
            None
        };
    
        Ok(AstNode::IfStatement {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
        })
    }

    fn expression_statement(&mut self) -> Result<AstNode, String> {
        let expr = self.expression()?;
        self.consume(&TokenType::Semicolon, "Ожидается ';' после выражения")?;
        Ok(expr)
    }

    fn expression(&mut self) -> Result<AstNode, String> {
        self.comparison()
    }

    fn comparison(&mut self) -> Result<AstNode, String> {
        let mut expr = self.additive()?;
        
        while self.check(&TokenType::DoubleEquals) || self.check(&TokenType::NotEquals) {
            let operator_type = self.peek().token_type.clone();
            self.advance();

            let operator = match operator_type {
                TokenType::DoubleEquals => "==".to_string(),
                TokenType::NotEquals => "!=".to_string(),
                _ => unreachable!(),
            };
            
            let right = self.additive()?;
            
            expr = AstNode::BinaryOp {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }

    fn additive(&mut self) -> Result<AstNode, String> {
        let mut expr = self.call_chain()?;
        
        while self.match_token(&TokenType::Concatenation) {
            let right = self.call_chain()?;
            
            expr = AstNode::BinaryOp {
                left: Box::new(expr),
                operator: "+".to_string(),
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }

    fn call_chain(&mut self) -> Result<AstNode, String> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(&TokenType::Dot) {
                let name_token = self.consume(&TokenType::Identifier(String::new()), "Ожидается имя свойства после '.'")?;
                let name = match &name_token.token_type {
                    TokenType::Identifier(n) => n.clone(),
                    _ => return Err("Невозможный случай при парсинге имени свойства".to_string()),
                };

                if self.check(&TokenType::LParen) {
                    self.advance();
                    let args = self.arguments()?;
                    let try_operator = self.match_token(&TokenType::TryOperator);
                    let unwrap_operator = self.match_token(&TokenType::UnwrapOperator);
                    expr = AstNode::FunctionCall {
                        object: Some(Box::new(expr)),
                        name,
                        args,
                        try_operator,
                        unwrap_operator,
                    };
                } else {
                    expr = AstNode::PropertyAccess {
                        object: Box::new(expr),
                        property: name,
                    };
                }
            } else if self.match_token(&TokenType::DoubleColon) {
                let object_name = match expr {
                    AstNode::Identifier(ref n) => n.clone(),
                    _ => return Err(format!("Ожидается идентификатор перед '::', получено: {}", expr)),
                };

                let fun_name_token = self.consume(&TokenType::Identifier(String::new()),
                "Ожидается имя функции после '::'")?;

                let fun_name = match &fun_name_token.token_type {
                    TokenType::Identifier(n) => n.clone(),
                    _ => return Err("Невозможный случай при парсинге имени функции плагина".to_string()),
                };

                self.consume(&TokenType::LParen,
                    "Ожидается '(' после имени функции плагина")?;
                let args = self.arguments()?;
                let try_operator = self.match_token(&TokenType::TryOperator);
                let unwrap_operator = self.match_token(&TokenType::UnwrapOperator);

                expr = AstNode::FunctionCall {
                    object: Some(Box::new(AstNode::Identifier(object_name))),
                    name: fun_name,
                    args,
                    try_operator,
                    unwrap_operator,
                };
            } else if self.match_token(&TokenType::LParen) && matches!(expr, AstNode::Identifier(_)) {
                let name = match &expr {
                    AstNode::Identifier(n) => n.clone(),
                    _ => return Err("Ожидается идентификатор перед '('".to_string()),
                };
                
                let args = self.arguments()?;

                let try_operator = self.match_token(&TokenType::TryOperator);
                let unwrap_operator = self.match_token(&TokenType::UnwrapOperator);

                expr = AstNode::FunctionCall {
                    object: None,
                    name,
                    args,
                    try_operator,
                    unwrap_operator,
                };
            } else {
                break;
            }

            if let AstNode::FunctionCall { object, name, args, try_operator, unwrap_operator } = &expr {
                let try_operator = self.match_token(&TokenType::TryOperator);
                let unwrap_operator = self.match_token(&TokenType::UnwrapOperator);

                if try_operator || unwrap_operator {
                    expr = AstNode::FunctionCall {
                        object: object.clone(),
                        name: name.clone(),
                        args: args.clone(),
                        try_operator,
                        unwrap_operator,
                    }
                }
            }
        }

        Ok(expr)
    }

    fn primary(&mut self) -> Result<AstNode, String> {
        if self.match_token(&TokenType::Identifier(String::new())) {
            let name = match &self.previous().token_type {
                TokenType::Identifier(n) => n.clone(),
                _ => return Err("Невозможный случай при парсинге идентификатора".to_string()),
            };
            Ok(AstNode::Identifier(name))
        } else if self.match_token(&TokenType::String(String::new())) {
            let value = match &self.previous().token_type {
                TokenType::String(s) => s.clone(),
                _ => return Err("Невозможный случай при парсинге строки".to_string()),
            };
            Ok(AstNode::StringLiteral(value))
        } else if self.match_token(&TokenType::Number(0)) {
            let value = match &self.previous().token_type {
                TokenType::Number(n) => *n,
                _ => return Err("Невозможный случай при парсинге числа".to_string()),
            };
            Ok(AstNode::NumberLiteral(value))
        } else {
            Err(format!("Ожидается выражение, получено {:?}", self.peek().token_type))
        }
    }

    fn arguments(&mut self) -> Result<Vec<Box<AstNode>>, String> {
        let mut args = Vec::new();

        if !self.check(&TokenType::RParen) {
            args.push(Box::new(self.expression()?));
            
            while self.match_token(&TokenType::Comma) {
                args.push(Box::new(self.expression()?));
            }
        }

        self.consume(&TokenType::RParen, "Ожидается ')' после списка аргументов")?;

        Ok(args)
    }
}

pub fn parse(input: &str) -> Result<AstNode, String> {
    debug!("Начало разбора файла...");
    
    let mut lexer = Lexer::new(input);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => {
            info!("Токенизация успешна, получено {} токенов", tokens.len());
            tokens
        },
        Err(e) => {
            error!("Ошибка токенизации: {}", e);
            return Err(e);
        }
    };  
    
    let mut parser = Parser::new(tokens);
    
    match parser.parse() {
        Ok(ast) => {
            info!("Парсинг успешен");
            Ok(ast)
        },
        Err(e) => {
            error!("Ошибка парсинга: {}", e);
            Err(e)
        }
    }
}