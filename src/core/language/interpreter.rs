use std::collections::HashMap; 
use std::fmt; 
use log::{error, trace, warn, debug};
use rustls::crypto::hash::Hash; 
use crate::core::servers::http_core::TlsConfig; 


#[allow(dead_code)] 
#[derive(Debug, Clone)] 
pub enum RouteAction { 
    
    VarDeclaration(String, Box<RouteAction>), 
    
    FunctionCall {
        object: Option<String>, 
        name: String, 
        args: Vec<Box<RouteAction>>, 
        try_operator: bool, 
        unwrap_operator: bool, 
    },
    
    Condition {
        check: Box<RouteAction>, 
        then_branch: Vec<Box<RouteAction>>, 
        else_branch: Option<Vec<Box<RouteAction>>>, 
    },
    
    StringLiteral(String), 
    
    NumberLiteral(i64), 
    
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

    GlobalErrorHandler {
        error_var: String, 
        body: Vec<Box<RouteAction>>, 
    },

    ErrorHandlerBlock {
        error_var: String, 
        body: Vec<Box<RouteAction>>, 
    },

    Error(String), 
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

    pub fn get_body(&self) -> String {
        self.body.clone().unwrap_or(String::new())
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

    pub fn status(&mut self, status: u16) {
        self.status = status;
    }
}


pub struct Database {}

impl Database {
    pub fn get_all() -> Result<String, String> {
        Ok(r#"[{"id": 1, "name": "User1"}, {"id": 2, "name": "User2"}]"#.to_string())
    }
    
    pub fn check() -> Result<bool, String> {
        Ok(true)
    }
    
    pub fn get(user_id: &str) -> Result<String, String> {
        if user_id == "0" {
            Err("Пользователь с id=0 не найден".to_string())
        } else {
            Ok(format!(r#"{{"id": {}, "name": "User{}"}}"#, user_id, user_id))
        }
    }
    
    pub fn add(user_id: &str, name: &str, password_hash: &str) -> Result<(), String> {
        trace!("Добавлен пользователь: id={}, name={}, password_hash={}", user_id, name, password_hash);
        Ok(())
    }
}


impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Database").finish()
    }
}


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


#[derive(Debug, Clone)] 
pub struct ErrorHandler {
    pub error_var: String, 
    pub actions: Vec<RouteAction>, 
}


#[derive(Debug, Clone)] 
pub struct RouteHandler {
    pub(crate) actions: Vec<RouteAction>, 
    pub(crate) error_handler: Option<ErrorHandler>, 
}

impl RouteHandler {
    fn new(actions: Vec<RouteAction>, error_handler: Option<ErrorHandler>) -> Self {
        RouteHandler { 
            actions,
            error_handler
        }
    }
    
    fn execute(&self, request: &mut Request, response: &mut Response, interpreter: &Interpreter) {
        let mut context = ExecutionContext::new();
        let mut error: Option<String> = None;
        
        for action in &self.actions {
            let result = self.execute_action(action, request, response, &mut context, interpreter);
            
            if let Err(err) = result {
                error = Some(err); 
                break; 
            }
            
            if response.is_sent {
                break; 
            }
        }
        
        if let Some(err) = error {
            if let Some(handler) = &self.error_handler {
                let mut err_context = context.clone();
                err_context.set_variable(&handler.error_var, err);
                
                for action in &handler.actions {
                    if let Err(_) = self.execute_action(action, request, response, &mut err_context, interpreter) {} 
                    
                    if response.is_sent {
                        break;
                    }
                }
            } else if let Some(global_handler) = &interpreter.global_error_handler {                
                let mut err_context = context.clone();
                err_context.set_variable(&global_handler.error_var, err);
                
                for action in &global_handler.actions {
                    if let Err(_) = self.execute_action(action, request, response, &mut err_context, interpreter) {}
                    
                    if response.is_sent {
                        break;
                    }
                }
            } else {
                response.status = 500;
                response.body(&format!("Internal Server Error: {}", err)).send(); 
            }
        }
    }
    
    fn execute_action(&self, action: &RouteAction, request: &mut Request, response: &mut Response, 
                    context: &mut ExecutionContext, interpreter: &Interpreter) -> Result<(), String> {
        
        if response.is_sent {
            return Ok(());
        }

        match action {
            RouteAction::VarDeclaration(name, value) => {
                let value_str = self.evaluate_expr(value, request, response, context)?;
                context.set_variable(name, value_str);
                Ok(()) 
            },
            RouteAction::FunctionCall { object, name, args, try_operator, unwrap_operator } => {
                let result = self.execute_function_call(object, name, args, request, response, context);
                
                if let Err(e) = &result {
                    if *try_operator {
                        debug!("Оператор try перехватил ошибку: {} в функции {}", e, name);
                        return Err(e.to_string()); 
                    }
                    if *unwrap_operator {
                        error!("Unwrap operator caught an error: {}", e); 
                        panic!("Unwrap operator caught an error: {}", e);
                    }
                    if !(*try_operator || *unwrap_operator) {
                        error!("Ошибка в функции {}: {}", name, e);
                        panic!("Error in function {}: {}", name, e);
                    }
                    return Err(e.to_string());
                }
                Ok(())
            },
            RouteAction::Condition { check, then_branch, else_branch } => {
                let condition_value = self.evaluate_expr(check, request, response, context)?;
                
                if condition_value == "true" || condition_value == "1" {
                    for action in then_branch {
                        if response.is_sent {
                            return Ok(());
                        }
                        if let Err(e) = self.execute_action(action, request, response, context, interpreter) {
                            return Err(e);
                        }
                    }
                } else if let Some(else_actions) = else_branch {
                    for action in else_actions {
                        if response.is_sent {
                            return Ok(());
                        }
                        if let Err(e) = self.execute_action(action, request, response, context, interpreter) {
                            return Err(e);
                        }
                    }
                }
                Ok(()) 
            },
            _ => Ok(()), 
        }
    }

    fn execute_function_call(&self, object: &Option<String>, name: &str, args: &Vec<Box<RouteAction>>,
        request: &mut Request, response: &mut Response, 
        context: &mut ExecutionContext) -> Result<String, String> {
        
        if let Some(obj_name) = object {
            let object_value = if let Some(var) = context.get_variable(obj_name) {
                var.clone() 
            } else {
                match obj_name.as_str() {   
                    "Database" => "Database".to_string(),
                    "Response" => "Response".to_string(),
                    "Request" => "Request".to_string(),
                    _ => return Err(format!("Объект не найден: {}", obj_name)), 
                }
            };

            match object_value.as_str() {
                "Database" => {
                    match name {
                        "get_all" => {
                            Database::get_all() 
                        },
                        "check" => {
                            Database::check().map(|v| v.to_string()) 
                        },
                        "get" => {
                            if args.len() == 1 {
                                let user_id = self.evaluate_expr(&args[0], request, response, context)?;
                                Database::get(&user_id)
                            } else {
                                Err("Метод Database.get требует один аргумент".to_string())
                            }
                        },
                        "add" => {
                            if args.len() >= 3 {
                                let arg0 = self.evaluate_expr(&args[0], request, response, context)?;
                                let arg1 = self.evaluate_expr(&args[1], request, response, context)?;
                                let arg2 = self.evaluate_expr(&args[2], request, response, context)?;
                                Database::add(&arg0, &arg1, &arg2).map(|_| "OK".to_string()) 
                            } else {
                                Err("Метод Database.add требует как минимум 3 аргумента".to_string())
                            }
                        },
                        _ => Err(format!("Метод не найден: Database.{}", name)),
                    }
                },
                "Response" => {
                    match name {
                        "body" => {
                            if args.len() == 1 {
                                let content = self.evaluate_expr(&args[0], request, response, context)?;
                                response.body(content.clone());
                                Ok(format!("{}", content).to_string()) 
                            } else {
                                Err("Метод Response.body требует один аргумент".to_string())
                            }
                        },
                        "send" => {
                            response.send();
                            Ok("".to_string())
                        },
                        "status" => {
                            if args.len() == 1 {
                                let status_str = self.evaluate_expr(&args[0], request, response, context)?;
                                if let Ok(status_code) = status_str.parse::<u16>() {
                                    response.status = status_code;
                                } else {
                                    warn!("Невозможно преобразовать статус в число: {}", status_str);
                                }
                                Ok(format!("{}", &status_str).to_string())
                            } else {
                                Err("Метод Response.status требует один аргумент".to_string())
                            }
                        },
                        "headers" => {
                            if args.len() == 2 {
                                let header_name = self.evaluate_expr(&args[0], request, response, context)?;
                                let header_value = self.evaluate_expr(&args[1], request, response, context)?;

                                response.headers.insert(header_name.clone(), header_value.clone());
                                Ok(format!("{}: {}", header_name, header_value).to_string())
                            } else {
                                Err("Метод Response.headers требует два аргумента".to_string())
                            }
                        },
                        _ => Err(format!("Метод не найден: response.{}", name)),
                    }
                },
                "Request" => {
                    match name {
                        "get_params" => {
                            if args.len() == 1 {
                                let param_name = self.evaluate_expr(&args[0], request, response, context)?;
                                Ok(request.get_params(&param_name)) 
                            } else {
                                Err("Метод request.get_params требует один аргумент".to_string())
                            }
                        },
                        "body" => {
                            if args.is_empty() {
                                if let Some(body) = &request.body {
                                    if body == "" {
                                        Err("Тело запроса пустое".to_string())
                                    } else {
                                        Ok(body.to_string())
                                    }
                                } else {
                                    Err("Тело запроса отсутствует".to_string())
                                }
                            } else {
                                Err("Метод request.body не принимает аргументы".to_string())
                            }
                        },
                        _ => Err(format!("Метод не найден: request.{}", name)),
                    }
                },
                _ => Err(format!("Объект не поддерживается: {}", object_value)),
            }
        } else {
            match name {
                "log_error" => {
                    if args.len() == 1 {
                        let message = self.evaluate_expr(&args[0], request, response, context)?;
                        error!("{}", message);
                        Ok("".to_string())
                    } else {
                        Err("Функция log_error требует один аргумент".to_string())
                    }
                },
                _ => Err(format!("Функция не найдена: {}", name)),
            }
        }
    }
    
    fn evaluate_expr(&self, expr: &RouteAction, request: &mut Request, response: &mut Response, context: &mut ExecutionContext) -> Result<String, String> {
        match expr {
            RouteAction::StringLiteral(value) => Ok(value.clone()),
            RouteAction::NumberLiteral(value) => Ok(value.to_string().clone()),
            RouteAction::Identifier(name) => {
                if let Some(value) = context.get_variable(name) {
                    Ok(value.clone())
                } else {
                    match name.as_str() {
                        "Request" => Ok("Request".to_string()),
                        "Response" => Ok("Response".to_string()),
                        "Database" => Ok("Database".to_string()),
                        _ => Ok(String::new()),
                    }
                }
            },
            RouteAction::FunctionCall { object, name, args, try_operator: _, unwrap_operator: _ } => {
                self.execute_function_call(object, name, args, request, response, context)
            },
            RouteAction::BinaryOp { left, operator, right } => {
                let left_value = self.evaluate_expr(left, request, response, context)?;
                let right_value = self.evaluate_expr(right, request, response, context)?;
                
                match operator.as_str() {
                    "==" => Ok((left_value == right_value).to_string()),
                    "!=" => Ok((left_value != right_value).to_string()),
                    "+" => Ok(format!("{}{}", left_value, right_value)),
                    "+=" => {
                        if let RouteAction::Identifier(name) = &**left { 
                            let current_value = context.get_variable(name)
                                                .cloned()
                                                .ok_or_else(|| format!("Переменная '{}' для '+=' не найдена", name))?;
                            let new_value = format!("{}{}", current_value, right_value);
                            context.set_variable(name, new_value.clone());
                            Ok(new_value) 
                        } else {
                             Err("Оператор += может использоваться только с идентификаторами слева".to_string())
                        }
                    },
                    _ => {
                        warn!("Неподдерживаемый оператор: {}", operator);
                        Err(format!("Not supported binary operator: {}", operator).to_string())
                    }
                }
            },
            _ => Ok(String::new()), 
        }
    }
}


#[derive(Debug)] 
pub struct Interpreter {
    pub routes: HashMap<String, (String, RouteHandler)>,
    pub tls_config: Option<TlsConfig>,
    pub global_error_handler: Option<ErrorHandler>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            routes: HashMap::new(), 
            tls_config: None, 
            global_error_handler: None, 
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
            crate::core::language::lexer::AstNode::ServerConfig { routes, tls_config, global_error_handler } => {                
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
                if let Some(handler) = global_error_handler {
                    match &**handler {
                        crate::core::language::lexer::AstNode::GlobalErrorHandler { error_var, body } => {
                            match self.convert_ast_to_actions(body) {
                                Ok(actions) => {
                                    self.global_error_handler = Some(ErrorHandler {
                                        error_var: error_var.clone(),
                                        actions,
                                    });
                                },
                                Err(e) => {
                                    return Err(format!("Failed to convert GlobalErrorHandler body: {}", e));
                                }
                            }
                        },
                        _ => return Err("Ожидается глобальный обработчик ошибок".to_string()),
                    }
                }
                
                for route in routes {
                    self.interpret(route)?; 
                }
                Ok(()) 
            },
            crate::core::language::lexer::AstNode::Route { path, method, body, on_error } => {
                trace!("Обработка маршрута: {} {}", method, path); 
                trace!("Тип тела маршрута: {:?}", body); 
                
                let actions = match self.convert_ast_to_actions(body) {
                    Ok(actions) => actions, 
                    Err(e) => {
                        error!("Ошибка преобразования тела маршрута: {}", e);
                        return Err(e);
                    }
                };

                let error_handler = if let Some(on_err) = on_error {
                    match &**on_err {
                        crate::core::language::lexer::AstNode::ErrorHandlerBlock { error_var, body } => {
                            let actions = self.convert_ast_to_actions(body)?;
                            Some(ErrorHandler {
                                error_var: error_var.clone(),
                                actions,
                            })
                        },
                        _ => return Err("Ожидается блок обработки ошибок".to_string()),
                    }
                } else {
                    None
                };
                
                let path_clone = path.clone();
                let method_clone = method.clone();
                
                let route_handler = RouteHandler::new(actions, error_handler);
                
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
            _ => {
                warn!("Неожиданный тип узла в convert_ast_to_actions: {:?}", node);
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
            crate::core::language::lexer::AstNode::FunctionCall { object, name, args, try_operator, unwrap_operator } => {
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
                    try_operator: *try_operator, 
                    unwrap_operator: *unwrap_operator,
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
            crate::core::language::lexer::AstNode::FunctionCall { object, name, .. } => { 
                if let Some(obj) = object {
                    let obj_name = self.extract_identifier_name(obj)?;
                    Ok(format!("{}.{}", obj_name, name))
                } else {
                    Ok(name.clone())
                }
            },
            crate::core::language::lexer::AstNode::BinaryOp { .. } => { 
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
            crate::core::language::lexer::AstNode::NumberLiteral(value) => {
                Ok(RouteAction::NumberLiteral(*value)) 
            }
            crate::core::language::lexer::AstNode::Identifier(name) => {
                Ok(RouteAction::Identifier(name.clone()))
            },
            crate::core::language::lexer::AstNode::FunctionCall { object, name, args, try_operator, unwrap_operator } => {
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
                    try_operator: *try_operator,
                    unwrap_operator: *unwrap_operator,
                })
            },
            crate::core::language::lexer::AstNode::PropertyAccess { object, property } => {
                let obj_action = self.convert_expression_to_action(object)?;
                Ok(RouteAction::PropertyAccess {
                    object: Box::new(obj_action), 
                    property: property.clone(),
                })
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
            handler.execute(&mut request, &mut response, self);
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
            global_error_handler: self.global_error_handler.clone(), 
        }
    }
}


pub fn handle_request(
    interpreter: &Interpreter,
    method: &str,
    path: &str,
    params: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: Option<String>,
) -> Response {
    let mut request = Request::new();
    request.params = params; 
    request.headers = headers;
    request.body = body;
    
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
                        
                        handler.execute(&mut request, &mut response, interpreter);

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
                handler.execute(&mut request, &mut response, interpreter);
                 if !response.headers.contains_key("Content-Type") {
                    response.headers.insert(
                        "Content-Type".to_string(),
                        "text/html; charset=utf-8".to_string(),
                    );
                 }
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