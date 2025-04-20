use std::collections::HashMap;
use std::fmt;
use log::{error, trace, warn, debug};
use crate::core::servers::http_core::TlsConfig;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum RouteAction {
    VarDeclaration(String, Box<RouteAction>),
    FunctionCall {
        object: Option<String>,
        name: String,
        args: Vec<Box<RouteAction>>,
    },
    Condition {
        check: Box<RouteAction>,
        then_branch: Vec<Box<RouteAction>>,
        else_branch: Option<Vec<Box<RouteAction>>>,
    },
    StringLiteral(String),
    Identifier(String),
    BinaryOp {
        left: Box<RouteAction>,
        operator: String,
        right: Box<RouteAction>,
    },
    PropertyAccess {
        object: Box<RouteAction>,
        property: String,
    },
}

pub struct Request {
    pub params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl Request {
    pub fn new() -> Self {
        Request {
            params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        }
    }
    
    pub fn get_params(&self, name: &str) -> String {
        self.params.get(name).unwrap_or(&String::new()).clone()
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("params", &self.params)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .finish()
    }
}

pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub is_sent: bool,
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("is_sent", &self.is_sent)
            .finish()
    }
}

impl Response {
    pub fn new() -> Self {
        Response {
            status: 200,
            headers: HashMap::new(),
            body: None,
            is_sent: false,
        }
    }
    
    pub fn body(&mut self, content: impl Into<String>) -> &mut Self {
        self.body = Some(content.into());
        self
    }
    
    pub fn send(&mut self) {
        self.is_sent = true;
    }
}

// Заглушка для базы
pub struct Database {}

impl Database {
    pub fn get_all() -> String {
        r#"[{"id": 1, "name": "User1"}, {"id": 2, "name": "User2"}]"#.to_string()
    }
    
    pub fn check() -> bool {
        true
    }
    
    pub fn get(user_id: &str) -> String {
        format!(r#"{{"id": {}, "name": "User{}"}}"#, user_id, user_id)
    }
    
    pub fn add(user_id: &str, name: &str, password_hash: &str) {
        trace!("Добавлен пользователь: id={}, name={}, password_hash={}", user_id, name, password_hash);
    }
}

impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Database").finish()
    }
}

// Контекст выполнения для хранения переменных
#[derive(Debug, Clone)]
struct ExecutionContext {
    variables: HashMap<String, String>,
}

impl ExecutionContext {
    fn new() -> Self {
        ExecutionContext {
            variables: HashMap::new(),
        }
    }
    
    fn set_variable(&mut self, name: &str, value: String) {
        self.variables.insert(name.to_string(), value);
    }
    
    fn get_variable(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }
}

// Структура для маршрута с собственными данными
#[derive(Debug, Clone)]
pub struct RouteHandler {
    pub(crate) actions: Vec<RouteAction>,
}

impl RouteHandler {
    fn new(actions: Vec<RouteAction>) -> Self {
        RouteHandler { actions }
    }
    
    fn execute(&self, request: &mut Request, response: &mut Response) {
        let mut context = ExecutionContext::new();
        
        for action in &self.actions {
            self.execute_action(action, request, response, &mut context);
            
            if response.is_sent {
                break;
            }
        }
    }
    
    fn execute_action(&self, action: &RouteAction, request: &mut Request, response: &mut Response, context: &mut ExecutionContext) {
        match action {
            RouteAction::VarDeclaration(name, value) => {
                let value_str = self.evaluate_expr(value, request, response, context);
                context.set_variable(name, value_str);
            },
            RouteAction::FunctionCall { object, name, args } => {
                if let Some(obj_name) = object {
                    let object_value = if let Some(var) = context.get_variable(obj_name) {
                        var.clone()
                    } else {
                        match obj_name.as_str() {   
                            "request" => "request".to_string(),
                            "response" => "response".to_string(),
                            "Database" => "Database".to_string(),
                            _ => String::new(),
                        }
                    };
                    
                    match object_value.as_str() {
                        "Database" => {
                            match name.as_str() {
                                "get_all" => {
                                    let result = Database::get_all();
                                    if name == "get_all" {
                                    }
                                    // there will be support for using get_all function
                                },
                                "check" => {
                                    let result = Database::check().to_string();
                                    // there will be support for using check function
                                },
                                "get" => {
                                    if args.len() == 1 {
                                        let user_id = self.evaluate_expr(&args[0], request, response, context);
                                        let result = Database::get(&user_id);
                                        // there will be support for using get function
                                    }
                                },
                                "add" => {
                                    if args.len() >= 3 {
                                        let arg0 = self.evaluate_expr(&args[0], request, response, context);
                                        let arg1 = self.evaluate_expr(&args[1], request, response, context);
                                        let arg2 = self.evaluate_expr(&args[2], request, response, context);
                                        Database::add(&arg0, &arg1, &arg2);
                                    }
                                },
                                _ => {},
                            }
                        },
                        "response" => {
                            match name.as_str() {
                                "body" => {
                                    if args.len() == 1 {
                                        let content = self.evaluate_expr(&args[0], request, response, context);
                                        response.body(content);
                                    }
                                },
                                "send" => {
                                    response.send();
                                },
                                _ => {},
                            }
                        },
                        "request" => {
                            match name.as_str() {
                                "get_params" => {
                                    if args.len() == 1 {
                                        let param_name = self.evaluate_expr(&args[0], request, response, context);
                                        let value = request.get_params(&param_name);
                                        // there will be support for using the value
                                    }
                                },
                                _ => {},
                            }
                        },
                        _ => {},
                    }
                }
            },
            RouteAction::Condition { check, then_branch, else_branch } => {
                let condition_value = self.evaluate_expr(check, request, response, context);
                
                if condition_value == "true" || condition_value == "1" {
                    for action in then_branch {
                        self.execute_action(action, request, response, context);
                        if response.is_sent {
                            break;
                        }
                    }
                } else if let Some(else_actions) = else_branch {
                    for action in else_actions {
                        self.execute_action(action, request, response, context);
                        if response.is_sent {
                            break;
                        }
                    }
                }
            },
            _ => {},
        }
    }
    
    fn evaluate_expr(&self, expr: &RouteAction, request: &mut Request, response: &mut Response, context: &mut ExecutionContext) -> String {
        match expr {
            RouteAction::StringLiteral(value) => value.clone(),
            RouteAction::Identifier(name) => {
                if let Some(value) = context.get_variable(name) {
                    value.clone()
                } else {
                    match name.as_str() {
                        "request" => "request".to_string(),
                        "response" => "response".to_string(),
                        "Database" => "Database".to_string(),
                        _ => String::new(),
                    }
                }
            },
            RouteAction::FunctionCall { object, name, args } => {
                if let Some(obj_name) = object {
                    let object_value = if let Some(var) = context.get_variable(obj_name) {
                        var.clone()
                    } else {
                        match obj_name.as_str() {
                            "request" => "request".to_string(),
                            "response" => "response".to_string(),
                            "Database" => "Database".to_string(),
                            _ => String::new(),
                        }
                    };
                    
                    match object_value.as_str() {
                        "Database" => {
                            match name.as_str() {
                                "get_all" => Database::get_all(),
                                "check" => Database::check().to_string(),
                                "get" => {
                                    if args.len() == 1 {
                                        let user_id = self.evaluate_expr(&args[0], request, response, context);
                                        Database::get(&user_id)
                                    } else {
                                        String::new()
                                    }
                                },
                                "add" => {
                                    if args.len() >= 3 {
                                        let arg0 = self.evaluate_expr(&args[0], request, response, context);
                                        let arg1 = self.evaluate_expr(&args[1], request, response, context);
                                        let arg2 = self.evaluate_expr(&args[2], request, response, context);
                                        Database::add(&arg0, &arg1, &arg2);
                                        "OK".to_string()
                                    } else {
                                        String::new()
                                    }
                                },
                                _ => String::new(),
                            }
                        },
                        "request" => {
                            match name.as_str() {
                                "get_params" => {
                                    if args.len() == 1 {
                                        let param_name = self.evaluate_expr(&args[0], request, response, context);
                                        request.get_params(&param_name)
                                    } else {
                                        String::new()
                                    }
                                },
                                _ => String::new(),
                            }
                        },
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            },
            RouteAction::BinaryOp { left, operator, right } => {
                let left_value = self.evaluate_expr(left, request, response, context);
                let right_value = self.evaluate_expr(right, request, response, context);
                
                match operator.as_str() {
                    "==" => {
                        if left_value == right_value {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    },
                    "!=" => {
                        if left_value != right_value {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    },
                    _ => {
                        warn!("Неподдерживаемый оператор: {}", operator);
                        "false".to_string()
                    }
                }
            },
            _ => String::new(),
        }
    }
}

#[derive(Debug)]
pub struct Interpreter {
    pub routes: HashMap<String, (String, RouteHandler)>,
    pub tls_config: Option<TlsConfig>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            routes: HashMap::new(),
            tls_config: None,
        }
    }
    
    pub fn interpret(&mut self, ast: &crate::core::language::lexer::AstNode) -> Result<(), String> {
        match ast {
            crate::core::language::lexer::AstNode::Program(statements) => {
                for stmt in statements {
                    match self.interpret(stmt) {
                        Ok(_) => {},
                        Err(e) => {
                            error!("Ошибка интерпретации: {}", e);
                            warn!("Тип узла: {:?}", stmt);
                            return Err(e);
                        }
                    }
                }
                Ok(())
            },
            crate::core::language::lexer::AstNode::ServerConfig { routes, tls_config } => {
                // Обрабатываем TLS конфигурацию
                if let Some(tls) = tls_config {
                    match &**tls {
                        crate::core::language::lexer::AstNode::TlsConfig { enabled, cert_path, key_path } => {
                            if *enabled {
                                self.tls_config = Some(TlsConfig {
                                    enabled: *enabled,
                                    cert_path: cert_path.clone(),
                                    key_path: key_path.clone(),
                                });
                                debug!("Включен TLS с сертификатами: cert={}, key={}", cert_path, key_path);
                            } else {
                                self.tls_config = None;
                                debug!("TLS отключен");
                            }
                        },
                        _ => return Err("Ожидается TLS конфигурация".to_string()),
                    }
                }
                
                // Обрабатываем все маршруты
                for route in routes {
                    self.interpret(route)?;
                }
                Ok(())
            },
            crate::core::language::lexer::AstNode::Route { path, method, body } => {
                trace!("Обработка маршрута: {} {}", method, path);
                
                trace!("Тип тела маршрута: {:?}", body);
                
                let actions = match self.convert_ast_to_actions(body) {
                    Ok(actions) => actions,
                    Err(e) => {
                        error!("Ошибка преобразования тела маршрута: {}", e);
                        return Err(e);
                    }
                };
                
                let path_clone = path.clone();
                let method_clone = method.clone();
                
                let route_handler = RouteHandler::new(actions);
                
                self.routes.insert(
                    format!("{}:{}", method_clone, path_clone), 
                    (path_clone, route_handler)
                );
                
                Ok(())
            },
            _ => {
                warn!("Неожиданный тип узла в interpret: {:?}", ast);
                Err("Ожидается программа, ServerConfig или маршрут на верхнем уровне".to_string())
            },
        }
    }
    
    fn convert_ast_to_actions(&self, node: &crate::core::language::lexer::AstNode) -> Result<Vec<RouteAction>, String> {
        match node {
            crate::core::language::lexer::AstNode::Block(statements) => {
                let mut actions = Vec::new();
                for stmt in statements {
                    let action = self.convert_statement_to_action(stmt)?;
                    actions.push(action);
                }
                Ok(actions)
            },
            crate::core::language::lexer::AstNode::IfStatement { condition, then_branch, else_branch } => {
                let mut actions = Vec::new();
                actions.push(self.convert_statement_to_action(node)?);
                Ok(actions)
            },
            _ => {
                println!("Неожиданный тип узла в convert_ast_to_actions: {:?}", node);
                Err("Ожидается блок кода".to_string())
            },
        }
    }
    
    fn convert_statement_to_action(&self, node: &crate::core::language::lexer::AstNode) -> Result<RouteAction, String> {
        match node {
            crate::core::language::lexer::AstNode::VarDeclaration { name, value } => {
                let value_action = self.convert_expression_to_action(value)?;
                Ok(RouteAction::VarDeclaration(name.clone(), Box::new(value_action)))
            },
            crate::core::language::lexer::AstNode::IfStatement { condition, then_branch, else_branch } => {
                let condition_action = self.convert_expression_to_action(condition)?;
                
                let then_actions = self.convert_ast_to_actions(then_branch)?;
                let then_boxed = then_actions.into_iter().map(Box::new).collect();
                
                let else_boxed = if let Some(else_stmt) = else_branch {
                    match &**else_stmt {
                        crate::core::language::lexer::AstNode::IfStatement { .. } => {
                            let else_if_action = self.convert_statement_to_action(else_stmt)?;
                            Some(vec![Box::new(else_if_action)])
                        },
                        _ => {
                            let else_actions = self.convert_ast_to_actions(else_stmt)?;
                            Some(else_actions.into_iter().map(Box::new).collect())
                        }
                    }
                } else {
                    None
                };
                
                Ok(RouteAction::Condition {
                    check: Box::new(condition_action),
                    then_branch: then_boxed,
                    else_branch: else_boxed,
                })
            },
            crate::core::language::lexer::AstNode::FunctionCall { object, name, args } => {
                let mut action_args = Vec::new();
                for arg in args {
                    let arg_action = self.convert_expression_to_action(arg)?;
                    action_args.push(Box::new(arg_action));
                }
                
                let object_name = if let Some(obj) = object {
                    Some(self.extract_identifier_name(obj)?)
                } else {
                    None
                };
                
                Ok(RouteAction::FunctionCall {
                    object: object_name,
                    name: name.clone(),
                    args: action_args,
                })
            },
            _ => self.convert_expression_to_action(node),
        }
    }
    
    fn extract_identifier_name(&self, node: &crate::core::language::lexer::AstNode) -> Result<String, String> {
        match node {
            crate::core::language::lexer::AstNode::Identifier(name) => Ok(name.clone()),
            crate::core::language::lexer::AstNode::PropertyAccess { object, property } => {
                let obj_name = self.extract_identifier_name(object)?;
                Ok(format!("{}.{}", obj_name, property))
            },
            crate::core::language::lexer::AstNode::FunctionCall { object, name, args } => {
                if let Some(obj) = object {
                    let obj_name = self.extract_identifier_name(obj)?;
                    Ok(format!("{}.{}", obj_name, name))
                } else {
                    Ok(name.clone())
                }
            },
            crate::core::language::lexer::AstNode::BinaryOp { left, operator, right } => {
                // There will be support for binary operations in the identifier
                Err("Бинарная операция не может быть использована как идентификатор!".to_string())
            },
            _ => Err(format!("Ожидается идентификатор, получено: {:?}", node).to_string()),
        }
    }
    
    fn convert_expression_to_action(&self, node: &crate::core::language::lexer::AstNode) -> Result<RouteAction, String> {
        match node {
            crate::core::language::lexer::AstNode::StringLiteral(value) => {
                Ok(RouteAction::StringLiteral(value.clone()))
            },
            crate::core::language::lexer::AstNode::Identifier(name) => {
                Ok(RouteAction::Identifier(name.clone()))
            },
            crate::core::language::lexer::AstNode::FunctionCall { object, name, args } => {
                let mut action_args = Vec::new();
                for arg in args {
                    let arg_action = self.convert_expression_to_action(arg)?;
                    action_args.push(Box::new(arg_action));
                }
                
                let object_name = if let Some(obj) = object {
                    Some(self.extract_identifier_name(obj)?)
                } else {
                    None
                };
                
                Ok(RouteAction::FunctionCall {
                    object: object_name,
                    name: name.clone(),
                    args: action_args,
                })
            },
            crate::core::language::lexer::AstNode::PropertyAccess { object, property } => {
                let obj_name = self.extract_identifier_name(object)?;
                Ok(RouteAction::Identifier(obj_name))
            },
            crate::core::language::lexer::AstNode::BinaryOp { left, operator, right } => {
                let left_action = self.convert_expression_to_action(left)?;
                let right_action = self.convert_expression_to_action(right)?;

                Ok(RouteAction::BinaryOp {
                    left: Box::new(left_action),
                    operator: operator.clone(),
                    right: Box::new(right_action),
                })
            },
            _ => Err(format!("Неподдерживаемое выражение: {:?}", node)),
        }
    }
    
    pub fn handle_request(&self, method: &str, path: &str, params: HashMap<String, String>) -> Response {
        let mut request = Request::new();
        request.params = params;
        
        let mut response = Response::new();
        
        if let Some((_, handler)) = self.routes.get(&format!("{}:{}", method, path)) {
            handler.execute(&mut request, &mut response);
        } else {
            response.status = 404;
            response.body("Not Found").send();
        }
        
        response
    }
}

impl Clone for Interpreter {
    fn clone(&self) -> Self {
        let mut new_routes = HashMap::new();
        for (key, (path, handler)) in &self.routes {
            new_routes.insert(key.clone(), (path.clone(), handler.clone()));
        }
        
        Interpreter {
            routes: new_routes,
            tls_config: self.tls_config.clone(),
        }
    }
}

pub fn handle_request(interpreter: &Interpreter, method: &str, path: &str, params: HashMap<String, String>) -> Response {
    let mut request = Request::new();
    request.params = params;
    
    let mut response = Response::new();
    
    for (route_key, (route_path, handler)) in &interpreter.routes {
        if route_key.starts_with(&format!("{}:", method)) {
            if route_path.contains('{') && route_path.contains('}') {
                let route_parts: Vec<&str> = route_path.split('/').collect();
                let request_parts: Vec<&str> = path.split('/').collect();
                
                if route_parts.len() == request_parts.len() {
                    let mut match_found = true;
                    let mut local_params = HashMap::new();
                    
                    for (i, (route_part, request_part)) in route_parts.iter().zip(request_parts.iter()).enumerate() {
                        if route_part.starts_with('{') && route_part.ends_with('}') {
                            let param_name = &route_part[1..route_part.len()-1];
                            local_params.insert(param_name.to_string(), request_part.to_string());
                        } else if route_part != request_part {
                            match_found = false;
                            break;
                        }
                    }
                    
                    if match_found {
                        for (k, v) in local_params {
                            request.params.insert(k, v);
                        }
                        
                        handler.execute(&mut request, &mut response);

                        if !response.headers.contains_key("Content-Type") {
                            response.headers.insert(
                                "Content-Type".to_string(),
                                "text/html; charset=utf-8".to_string(),
                            );
                        }

                        return response;
                    }
                }
            } else if route_path == path {
                handler.execute(&mut request, &mut response);
                return response;
            }
        }
    }
    
    response.status = 404;
    response.body("Not Found").send();
    response.headers.insert(
        "Content-Type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    response
}