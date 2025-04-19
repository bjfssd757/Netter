use crate::core::language::operators::{
    Token,
    TokenType,
};
use crate::core::language::lexer::{
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

        while !self.is_at_end() {
            statements.push(Box::new(self.route()?));
        }

        Ok(AstNode::Program(statements))
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

        self.consume(&TokenType::Semicolon, "Ожидается ';' после блока маршрута")?;

        Ok(AstNode::Route {
            path,
            method,
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
        self.equality()
    }

    fn equality(&mut self) -> Result<AstNode, String> {
        let mut expr = self.call_chain()?;
        
        while self.match_token(&TokenType::DoubleEquals) {
            let right = self.call_chain()?;
            
            expr = AstNode::BinaryOp {
                left: Box::new(expr),
                operator: "==".to_string(),
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
                    expr = AstNode::FunctionCall {
                        object: Some(Box::new(expr)),
                        name,
                        args,
                    };
                } else {
                    expr = AstNode::PropertyAccess {
                        object: Box::new(expr),
                        property: name,
                    };
                }
            } else if self.match_token(&TokenType::LParen) && matches!(expr, AstNode::Identifier(_)) {
                let name = match &expr {
                    AstNode::Identifier(n) => n.clone(),
                    _ => return Err("Ожидается идентификатор перед '('".to_string()),
                };
                
                let args = self.arguments()?;
                expr = AstNode::FunctionCall {
                    object: None,
                    name,
                    args,
                };
            } else {
                break;
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