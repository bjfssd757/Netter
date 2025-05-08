use crate::language::lexer::AstNode;
use crate::servers::http_core::TlsConfig;
use base64::Engine;
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use crate::servers::http_core::HttpBodyVariant;
use libloading::{Library, Symbol};
use serde_json;
use std::ffi::CString;
use std::os::raw::c_char;

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
    Error(String),
}

#[derive(Debug)]
pub struct Request {
    pub params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: HttpBodyVariant,
}

impl Request {
    pub fn new() -> Self {
        Request {
            params: HashMap::new(),
            headers: HashMap::new(),
            body: HttpBodyVariant::Empty,
        }
    }
    pub fn get_params(&self, name: &str) -> String {
        self.params.get(name).cloned().unwrap_or_default()
    }
    pub fn get_body(&self) -> String {
        match &self.body {
            HttpBodyVariant::Empty => "".to_string(),
            HttpBodyVariant::Text(text) => text.clone(),
            HttpBodyVariant::Bytes(_) => "[Binary Body - Use body_base64() for content]".to_string(),
        }
    }
    pub fn get_body_as_base64(&self) -> String {
        match &self.body {
            HttpBodyVariant::Text(s) => base64::engine::general_purpose::STANDARD.encode(s.as_bytes()),
            HttpBodyVariant::Bytes(bytes_vec) => base64::engine::general_purpose::STANDARD.encode(bytes_vec),
            HttpBodyVariant::Empty => "".to_string(),
        }
    }
    pub fn is_body_binary(&self) -> bool {
        matches!(&self.body, HttpBodyVariant::Bytes(_))
    }
}

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub is_sent: bool,
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
        if !self.headers.contains_key("Content-Type") && self.body.is_some() {
            self.headers.insert(
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            );
        }
    }
    pub fn status(&mut self, status: u16) {
        self.status = status;
    }
    pub fn set_header(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }
}

#[derive(Debug)]
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
            Ok(format!(
                r#"{{"id": {}, "name": "User{}"}}"#,
                user_id, user_id
            ))
        }
    }
    pub fn add(user_id: &str, name: &str, password_hash: &str) -> Result<(), String> {
        trace!(
            "Добавлен пользователь: id={}, name={}, password_hash={}",
            user_id, name, password_hash
        );
        Ok(())
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
pub struct Configuration {
    pub config_type: String,
    pub host: String,
    pub port: String,
}

#[derive(Debug)]
pub struct Interpreter {
    pub routes: HashMap<String, (String, RouteHandler)>,
    pub tls_config: Option<TlsConfig>,
    pub global_error_handler: Option<ErrorHandler>,
    pub configuration: Option<Configuration>,
    pub loaded_plugins: HashMap<String, Library>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            routes: HashMap::new(),
            tls_config: None,
            global_error_handler: None,
            configuration: None,
            loaded_plugins: HashMap::new(),
        }
    }

    pub fn interpret(&mut self, ast: &crate::language::lexer::AstNode) -> Result<(), String> {
        debug!("Начало интерпретации AST узла: {:?}", ast);
        let statements_ref: &Vec<Box<AstNode>>;
        let mut server_tls_config_opt: Option<&Box<AstNode>> = None;
        let mut server_global_handler_opt: Option<&Box<AstNode>> = None;
        let mut server_config_block_opt: Option<&Box<AstNode>> = None;
        let temp_statements: Vec<Box<AstNode>>;

        match ast {
            AstNode::Program(stmts) => {
                debug!("Интерпретация Program с {} стейтментами", stmts.len());
                statements_ref = stmts;
            }
            AstNode::ServerConfig {
                routes,
                tls_config,
                global_error_handler,
                config_block,
            } => {
                debug!("Интерпретация ServerConfig с {} стейтментами", routes.len());
                statements_ref = routes;
                server_tls_config_opt = tls_config.as_ref();
                server_global_handler_opt = global_error_handler.as_ref();
                server_config_block_opt = config_block.as_ref();
            }
            AstNode::Import { .. } => {
                debug!("Интерпретация одиночного Import узла");
                temp_statements = vec![Box::new(ast.clone())];
                statements_ref = &temp_statements;
            }
            _ => {
                error!(
                    "Неожиданный тип узла верхнего уровня в interpret: {:?}",
                    ast
                );
                return Err(format!(
                    "Ожидается Program или ServerConfig на верхнем уровне, получено: {:?}",
                    ast
                ));
            }
        };

        if let Some(ast_node) = server_tls_config_opt {
            match &**ast_node {
                AstNode::TlsConfig {
                    enabled,
                    cert_path,
                    key_path,
                } => {
                    self.tls_config = Some(TlsConfig {
                        enabled: *enabled,
                        cert_path: cert_path.clone(),
                        key_path: key_path.clone(),
                    });
                    debug!("Конфигурация TLS установлена: enabled={}", *enabled);
                }
                _ => return Err("Ожидался узел TlsConfig внутри ServerConfig".to_string()),
            }
        }
        if let Some(ast_node) = server_global_handler_opt {
            match &**ast_node {
                AstNode::GlobalErrorHandler { error_var, body } => {
                    let actions = self.convert_ast_to_actions(body.as_ref())?;
                    self.global_error_handler = Some(ErrorHandler {
                        error_var: error_var.clone(),
                        actions,
                    });
                    debug!(
                        "Глобальный обработчик ошибок установлен для переменной '{}'",
                        error_var
                    );
                }
                _ => return Err("Ожидался узел GlobalErrorHandler внутри ServerConfig".to_string()),
            }
        }
        if let Some(ast_node) = server_config_block_opt {
            match &**ast_node {
                AstNode::ConfigBlock {
                    config_type,
                    host,
                    port,
                } => {
                    self.configuration = Some(Configuration {
                        config_type: config_type.clone(),
                        host: host.clone(),
                        port: port.clone(),
                    });
                    debug!(
                        "Конфигурация сервера установлена: type={}, host={}, port={}",
                        config_type, host, port
                    );
                }
                _ => return Err("Ожидался узел ConfigBlock внутри ServerConfig".to_string()),
            }
        }

        debug!("Начало загрузки плагинов...");
        for stmt in statements_ref {
            if let AstNode::Import { path, alias } = &**stmt {
                debug!("Загрузка плагина: '{}' из '{}'", alias, path);
                unsafe {
                    match Library::new(path) {
                        Ok(lib) => {
                            if self.loaded_plugins.contains_key(alias.as_str()) {
                                warn!("Переопределение плагина с псевдонимом: {}", alias);
                            }
                            self.loaded_plugins.insert(alias.clone(), lib);
                            debug!("Плагин '{}' успешно загружен.", alias);
                        }
                        Err(e) => {
                            let err_msg = format!(
                                "Критическая ошибка: Не удалось загрузить плагин '{}' из {}: {}",
                                alias, path, e
                            );
                            error!("{}", err_msg);
                            return Err(err_msg);
                        }
                    }
                }
            }
        }
        debug!(
            "Загрузка плагинов завершена. Загружено: {} плагинов.",
            self.loaded_plugins.len()
        );

        debug!("Начало обработки маршрутов...");
        for stmt in statements_ref {
            match &**stmt {
                AstNode::Import { .. } => continue,
                AstNode::Route {
                    path,
                    method,
                    body,
                    on_error,
                } => {
                    trace!("Интерпретация маршрута: {} {}", method, path);
                    let actions = self.convert_ast_to_actions(body.as_ref()).map_err(|e| {
                        format!(
                            "Ошибка конвертации тела маршрута {} {}: {}",
                            method, path, e
                        )
                    })?;

                    let error_handler = if let Some(on_err) = on_error {
                        match &**on_err {
                            AstNode::ErrorHandlerBlock { error_var, body } => {
                                let eh_actions =
                                    self.convert_ast_to_actions(body.as_ref()).map_err(|e| {
                                        format!(
                                            "Ошибка конвертации onError для маршрута {} {}: {}",
                                            method, path, e
                                        )
                                    })?;
                                Some(ErrorHandler {
                                    error_var: error_var.clone(),
                                    actions: eh_actions,
                                })
                            }
                            _ => {
                                return Err(format!(
                                    "Ожидался ErrorHandlerBlock для маршрута {} {}",
                                    method, path
                                ));
                            }
                        }
                    } else {
                        None
                    };

                    let route_handler = RouteHandler::new(actions, error_handler);
                    let route_key = format!("{}:{}", method, path);
                    if self.routes.contains_key(&route_key) {
                        warn!("Переопределение маршрута: {}", route_key);
                    }
                    debug!("Добавление обработчика для маршрута: {}", route_key);
                    self.routes.insert(route_key, (path.clone(), route_handler));
                }
                _ => {
                    warn!(
                        "Неожиданный тип узла в основном цикле обработки (пропускается): {:?}",
                        stmt
                    );
                }
            }
        }
        debug!(
            "Обработка маршрутов завершена. Всего маршрутов: {}",
            self.routes.len()
        );
        info!("Интерпретация конфигурации успешно завершена.");
        Ok(())
    }

    fn convert_ast_to_actions(&self, node: &AstNode) -> Result<Vec<RouteAction>, String> {
        match node {
            AstNode::Block(statements) => statements
                .iter()
                .map(|stmt| self.convert_statement_to_action(stmt))
                .collect(),
            _ => self
                .convert_statement_to_action(node)
                .map(|action| vec![action]),
        }
    }

    fn convert_statement_to_action(&self, node: &AstNode) -> Result<RouteAction, String> {
        trace!("Конвертация statement: {:?}", node);
        match node {
            AstNode::VarDeclaration { name, value } => self
                .convert_expression_to_action(value)
                .map(|v| RouteAction::VarDeclaration(name.clone(), Box::new(v))),
            AstNode::IfStatement {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_action = self.convert_expression_to_action(condition)?;
                let then_actions = self.convert_ast_to_actions(then_branch.as_ref())?;
                let then_boxed = then_actions.into_iter().map(Box::new).collect();
                let else_boxed = if let Some(else_stmt) = else_branch {
                    let else_actions = self.convert_ast_to_actions(else_stmt.as_ref())?;
                    Some(else_actions.into_iter().map(Box::new).collect())
                } else {
                    None
                };
                Ok(RouteAction::Condition {
                    check: Box::new(condition_action),
                    then_branch: then_boxed,
                    else_branch: else_boxed,
                })
            }
            AstNode::FunctionCall {
                object,
                name,
                args,
                try_operator,
                unwrap_operator,
            } => {
                let obj_name = if let Some(o) = object {
                    Some(self.extract_identifier_name(o)?)
                } else {
                    None
                };
                let route_args = args
                    .iter()
                    .map(|arg| self.convert_expression_to_action(arg).map(Box::new))
                    .collect::<Result<_, _>>()?;
                Ok(RouteAction::FunctionCall {
                    object: obj_name,
                    name: name.clone(),
                    args: route_args,
                    try_operator: *try_operator,
                    unwrap_operator: *unwrap_operator,
                })
            }
            AstNode::BinaryOp {
                left,
                operator,
                right,
            } if operator == "+=" => {
                let left_action = self.convert_expression_to_action(left)?;
                let right_action = self.convert_expression_to_action(right)?;
                Ok(RouteAction::BinaryOp {
                    left: Box::new(left_action),
                    operator: operator.clone(),
                    right: Box::new(right_action),
                })
            }
            _ => self.convert_expression_to_action(node),
        }
    }

    fn convert_expression_to_action(&self, node: &AstNode) -> Result<RouteAction, String> {
        trace!("Конвертация expression: {:?}", node);
        match node {
            AstNode::StringLiteral(s) => Ok(RouteAction::StringLiteral(s.clone())),
            AstNode::NumberLiteral(n) => Ok(RouteAction::NumberLiteral(*n)),
            AstNode::Identifier(id) => Ok(RouteAction::Identifier(id.clone())),
            AstNode::FunctionCall {
                object,
                name,
                args,
                try_operator,
                unwrap_operator,
            } => {
                let obj_name = if let Some(o) = object {
                    Some(self.extract_identifier_name(o)?)
                } else {
                    None
                };
                let route_args = args
                    .iter()
                    .map(|arg| self.convert_expression_to_action(arg).map(Box::new))
                    .collect::<Result<_, _>>()?;
                Ok(RouteAction::FunctionCall {
                    object: obj_name,
                    name: name.clone(),
                    args: route_args,
                    try_operator: *try_operator,
                    unwrap_operator: *unwrap_operator,
                })
            }
            AstNode::BinaryOp {
                left,
                operator,
                right,
            } => {
                let l = self.convert_expression_to_action(left)?;
                let r = self.convert_expression_to_action(right)?;
                Ok(RouteAction::BinaryOp {
                    left: Box::new(l),
                    operator: operator.clone(),
                    right: Box::new(r),
                })
            }
            AstNode::PropertyAccess { object, property } => {
                let obj = self.convert_expression_to_action(object)?;
                Ok(RouteAction::PropertyAccess {
                    object: Box::new(obj),
                    property: property.clone(),
                })
            }
            AstNode::Block(_) => Err(
                "Узел Block не может быть напрямую преобразован в выражение RouteAction"
                    .to_string(),
            ),
            AstNode::Route { .. } => {
                Err("Узел Route не может быть преобразован в выражение RouteAction".to_string())
            }
            AstNode::TlsConfig { .. } => {
                Err("Узел TlsConfig не может быть преобразован в выражение RouteAction".to_string())
            }
            AstNode::ServerConfig { .. } => Err(
                "Узел ServerConfig не может быть преобразован в выражение RouteAction".to_string(),
            ),
            AstNode::GlobalErrorHandler { .. } => Err(
                "Узел GlobalErrorHandler не может быть преобразован в выражение RouteAction"
                    .to_string(),
            ),
            AstNode::ErrorHandlerBlock { .. } => Err(
                "Узел ErrorHandlerBlock не может быть преобразован в выражение RouteAction"
                    .to_string(),
            ),
            AstNode::ConfigBlock { .. } => Err(
                "Узел ConfigBlock не может быть преобразован в выражение RouteAction".to_string(),
            ),
            AstNode::Import { .. } => {
                Err("Узел Import не может быть преобразован в выражение RouteAction".to_string())
            }
            AstNode::Program(_) => {
                Err("Узел Program не может быть преобразован в выражение RouteAction".to_string())
            }
            AstNode::VarDeclaration { .. } => Err(
                "Узел VarDeclaration не может быть преобразован в выражение RouteAction"
                    .to_string(),
            ),
            AstNode::IfStatement { .. } => Err(
                "Узел IfStatement не может быть преобразован в выражение RouteAction".to_string(),
            ),
        }
    }

    fn extract_identifier_name(
        &self,
        node: &crate::language::lexer::AstNode,
    ) -> Result<String, String> {
        match node {
            AstNode::Identifier(name) => Ok(name.clone()),
            _ => Err(format!("Ожидается идентификатор, получено: {:?}", node)),
        }
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
            configuration: self.configuration.clone(),
            loaded_plugins: HashMap::new(),
        }
    }
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
            error_handler,
        }
    }

    fn execute(&self, request: &mut Request, response: &mut Response, interpreter: &Interpreter) {
        let mut context = ExecutionContext::new();
        let mut error: Option<String> = None;
        debug!(
            "Начало выполнения маршрута. Действий: {}",
            self.actions.len()
        );
        for (index, action) in self.actions.iter().enumerate() {
            trace!("Выполнение действия {}: {:?}", index, action);
            let result = self.execute_action(action, request, response, &mut context, interpreter);
            if let Err(err_msg) = result {
                error = Some(err_msg);
                debug!(
                    "Ошибка при выполнении действия {}: {}",
                    index,
                    error.as_ref().unwrap()
                );
                break;
            }
            if response.is_sent {
                debug!("Ответ отправлен действием {}, прерывание.", index);
                break;
            }
        }
        if let Some(err_msg) = error {
            warn!("Произошла ошибка при выполнении маршрута: {}", err_msg);
            let mut error_handled = false;
            if let Some(handler) = &self.error_handler {
                debug!(
                    "Использование локального обработчика ошибок для переменной '{}'",
                    handler.error_var
                );
                let mut err_context = context.clone();
                err_context.set_variable(&handler.error_var, err_msg.clone());
                for (index, action) in handler.actions.iter().enumerate() {
                    trace!("Выполнение действия обработчика {}: {:?}", index, action);
                    if let Err(e) = self.execute_action(
                        action,
                        request,
                        response,
                        &mut err_context,
                        interpreter,
                    ) {
                        error!("Ошибка внутри локального обработчика ошибок: {}", e);
                    }
                    if response.is_sent {
                        debug!(
                            "Ответ отправлен действием обработчика {}, прерывание.",
                            index
                        );
                        error_handled = true;
                        break;
                    }
                }
                if !response.is_sent {
                    error_handled = true;
                    debug!("Локальный обработчик ошибок завершился без отправки ответа.");
                }
            }
            if !error_handled {
                if let Some(global_handler) = &interpreter.global_error_handler {
                    debug!(
                        "Использование глобального обработчика ошибок для переменной '{}'",
                        global_handler.error_var
                    );
                    let mut err_context = context.clone();
                    err_context.set_variable(&global_handler.error_var, err_msg.clone());
                    for (index, action) in global_handler.actions.iter().enumerate() {
                        trace!(
                            "Выполнение действия глоб. обработчика {}: {:?}",
                            index, action
                        );
                        if let Err(e) = self.execute_action(
                            action,
                            request,
                            response,
                            &mut err_context,
                            interpreter,
                        ) {
                            error!("Ошибка внутри глобального обработчика ошибок: {}", e);
                        }
                        if response.is_sent {
                            debug!(
                                "Ответ отправлен действием глоб. обработчика {}, прерывание.",
                                index
                            );
                            error_handled = true;
                            break;
                        }
                    }
                    if !response.is_sent {
                        error_handled = true;
                        debug!("Глобальный обработчик ошибок завершился без отправки ответа.");
                    }
                }
            }
            if !error_handled && !response.is_sent {
                error!(
                    "Ошибка не обработана ни локальным, ни глобальным обработчиком. Отправка 500."
                );
                response.status(500);
                response.body(&format!("Internal Server Error: {}", err_msg));
                response.send();
            }
        } else {
            debug!("Выполнение маршрута завершено успешно.");
        }
    }

    fn execute_action(
        &self,
        action: &RouteAction,
        request: &mut Request,
        response: &mut Response,
        context: &mut ExecutionContext,
        interpreter: &Interpreter,
    ) -> Result<(), String> {
        if response.is_sent {
            return Ok(());
        }
        match action {
            RouteAction::VarDeclaration(name, value_expr) => {
                trace!("Объявление переменной '{}'", name);
                let value_str =
                    self.evaluate_expr(value_expr, request, response, context, interpreter)?;
                debug!("Установка переменной '{}' = '{}'", name, value_str);
                context.set_variable(name, value_str);
                Ok(())
            }
            RouteAction::FunctionCall {
                object,
                name,
                args,
                try_operator,
                unwrap_operator,
            } => {
                trace!(
                    "Вызов функции/метода: {}.{}",
                    object.as_deref().unwrap_or("<global>"),
                    name
                );
                let result = self.execute_function_call(
                    object,
                    name,
                    args,
                    request,
                    response,
                    context,
                    interpreter,
                );
                match result {
                    Ok(value) => {
                        trace!(
                            "Успешный вызов {}.{}. Результат (игнорируется): {}",
                            object.as_deref().unwrap_or("<global>"),
                            name,
                            value
                        );
                        Ok(())
                    }
                    Err(e) => {
                        if *try_operator {
                            debug!(
                                "Оператор '?' перехватил ошибку при вызове {}.{}: {}",
                                object.as_deref().unwrap_or("<global>"),
                                name,
                                e
                            );
                            Err(e)
                        } else if *unwrap_operator {
                            error!(
                                "Оператор '!!' вызвал панику при ошибке в {}.{}: {}",
                                object.as_deref().unwrap_or("<global>"),
                                name,
                                e
                            );
                            panic!("Ошибка выполнения (unwrap !!): {}", e);
                        } else {
                            error!(
                                "Неперехваченная ошибка при вызове {}.{}: {}",
                                object.as_deref().unwrap_or("<global>"),
                                name,
                                e
                            );
                            Err(e)
                        }
                    }
                }
            }
            RouteAction::Condition {
                check,
                then_branch,
                else_branch,
            } => {
                trace!("Проверка условия: {:?}", check);
                let condition_value = self.evaluate_expr(check, request, response, context, interpreter)?;
                debug!("Результат условия: {}", condition_value);
                if condition_value == "true" || condition_value == "1" {
                    debug!("Выполнение ветки 'then'");
                    for action in then_branch {
                        if response.is_sent {
                            return Ok(());
                        }
                        if let Err(e) = self.execute_action(action, request, response, context, interpreter) {
                            return Err(e);
                        }
                    }
                } else if let Some(else_actions) = else_branch {
                    debug!("Выполнение ветки 'else'");
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
            }
            RouteAction::BinaryOp {
                left,
                operator,
                right,
            } if operator == "+=" => {
                if let RouteAction::Identifier(var_name) = &**left {
                    let right_value =
                        self.evaluate_expr(right, request, response, context, interpreter)?;
                    let current_value = context
                        .get_variable(var_name)
                        .cloned()
                        .ok_or_else(|| format!("Переменная '{}' для '+=' не найдена", var_name))?;
                    let new_value = format!("{}{}", current_value, right_value);
                    trace!("Обновление переменной '{}' += '{}' -> '{}'",
                        var_name, right_value, new_value
                    );
                    context.set_variable(var_name, new_value);
                    Ok(())
                } else {
                    Err("Оператор '+=' может использоваться только с идентификатором переменной слева".to_string())
                }
            }
            RouteAction::StringLiteral(_)
            | RouteAction::NumberLiteral(_)
            | RouteAction::Identifier(_)
            | RouteAction::BinaryOp { .. }
            | RouteAction::PropertyAccess { .. } => {
                match self.evaluate_expr(action, request, response, context, interpreter) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
            RouteAction::Error(msg) => Err(msg.clone()),
        }
    }

    fn execute_function_call(
        &self,
        object: &Option<String>,
        name: &str,
        args: &Vec<Box<RouteAction>>,
        request: &mut Request,
        response: &mut Response,
        context: &mut ExecutionContext,
        interpreter: &Interpreter,
    ) -> Result<String, String> {
        if let Some(alias) = object {
            if let Some(library) = interpreter.loaded_plugins.get(alias) {
                trace!("Диспетчеризация вызова плагина: {}::{}", alias, name);
                let mut evaluated_args = Vec::new();
                for arg_expr in args {
                    evaluated_args.push(self.evaluate_expr(
                        arg_expr,
                        request,
                        response,
                        context,
                        interpreter,
                    )?);
                }

                let args_json = serde_json::to_string(&evaluated_args).map_err(|e| {
                    format!("Ошибка сериализации аргументов для {}::{}: {}", alias, name, e)
                })?;
                trace!("Аргументы (JSON) для {}::{}: {}", alias, name, args_json);
        
                let c_name = CString::new(name.as_bytes()).map_err(|e| {
                    format!("Ошибка создания CString для имени функции {}::{}: {}", alias, name, e)
                })?;
                let c_args_json = CString::new(args_json).map_err(|e| {
                    format!("Ошибка создания CString для аргументов {}::{}: {}", alias, name, e)
                })?;
        
                type DispatchFuncSig = unsafe extern "C" fn(
                    func_name_ptr: *const c_char,
                    args_json_ptr: *const c_char,
                ) -> *mut c_char;
        
                let result_string = unsafe {
                    let dispatch_func: Symbol<DispatchFuncSig> =
                        library.get(b"__netter_dispatch").map_err(|e| {
                            format!("Функция диспетчера '__netter_dispatch' не найдена в плагине '{}': {}. Убедитесь, что netter_plugger::generate_dispatch_func!() вызван.", alias, e)
                        })?;

                    let result_ptr = dispatch_func(c_name.as_ptr(), c_args_json.as_ptr());

                    if result_ptr.is_null() {
                        return Err(format!("Функция диспетчера плагина {} вернула null для вызова {}", alias, name));
                    }
                    match CString::from_raw(result_ptr).into_string() {
                        Ok(s) => s,
                        Err(e) => {
                            return Err(format!("Ошибка конвертации результата диспетчера из плагина {} (вызов {}): {}",
                                alias, name, e
                            ));
                        }
                    }
                };
        
                trace!("Результат от диспетчера {} для {}: {}", alias, name, result_string);

                if let Some(ok_result) = result_string.strip_prefix("OK:") {
                    return Ok(ok_result.to_string());
                } else if let Some(err_msg) = result_string.strip_prefix("ERR:") {
                    return Err(err_msg.to_string());
                } else {
                    return Err(format!("Неверный формат ответа от диспетчера плагина {} (вызов {}): нет префикса 'OK:' или 'ERR:'",
                        alias, name
                    ));
                }
            }
        }

        if let Some(obj_name) = object {
            let object_kind = if let Some(_var_value) = context.get_variable(obj_name) {
                return Err(format!("Вызов методов у переменных ('{}') пока не поддерживается",
                    obj_name
                ));
            } else {
                match obj_name.as_str() {
                    "Database" => "Database",
                    "Response" => "Response",
                    "Request" => "Request",
                    _ => return Err(format!("Объект или плагин не найден: {}", obj_name)),
                }
            };
            match object_kind {
                "Database" => match name {
                    "get_all" => Database::get_all(),
                    "check" => Database::check().map(|v| v.to_string()),
                    "get" => {
                        if args.len() == 1 {
                            let user_id = self.evaluate_expr(
                                &args[0],
                                request,
                                response,
                                context,
                                interpreter,
                            )?;
                            Database::get(&user_id)
                        } else {
                            Err("Метод Database.get требует 1 аргумент".to_string())
                        }
                    }
                    "add" => {
                        if args.len() >= 3 {
                            let arg0 = self.evaluate_expr(
                                &args[0],
                                request,
                                response,
                                context,
                                interpreter,
                            )?;
                            let arg1 = self.evaluate_expr(
                                &args[1],
                                request,
                                response,
                                context,
                                interpreter,
                            )?;
                            let arg2 = self.evaluate_expr(
                                &args[2],
                                request,
                                response,
                                context,
                                interpreter,
                            )?;
                            Database::add(&arg0, &arg1, &arg2).map(|_| "OK".to_string())
                        } else {
                            Err("Метод Database.add требует 3 аргумента".to_string())
                        }
                    }
                    _ => Err(format!("Метод не найден: Database.{}", name)),
                },
                "Response" => match name {
                    "body" => {
                        if args.len() == 1 {
                            let content = self.evaluate_expr(
                                &args[0],
                                request,
                                response,
                                context,
                                interpreter,
                            )?;
                            response.body(content.clone());
                            Ok(content)
                        } else {
                            Err("Метод Response.body требует 1 аргумент".to_string())
                        }
                    }
                    "send" => {
                        response.send();
                        Ok("".to_string())
                    }
                    "status" => {
                        if args.len() == 1 {
                            let status_str = self.evaluate_expr(
                                &args[0],
                                request,
                                response,
                                context,
                                interpreter,
                            )?;
                            if let Ok(status_code) = status_str.parse::<u16>() {
                                response.status(status_code);
                                Ok(status_code.to_string())
                            } else {
                                Err(format!("Неверный статус код: {}", status_str))
                            }
                        } else {
                            Err("Метод Response.status требует 1 аргумент".to_string())
                        }
                    }
                    "headers" => {
                        if args.len() == 2 {
                            let header_name = self.evaluate_expr(
                                &args[0],
                                request,
                                response,
                                context,
                                interpreter,
                            )?;
                            let header_value = self.evaluate_expr(
                                &args[1],
                                request,
                                response,
                                context,
                                interpreter,
                            )?;
                            response
                                .headers
                                .insert(header_name.clone(), header_value.clone());
                            Ok(format!("{}: {}", header_name, header_value))
                        } else {
                            Err("Метод Response.headers требует 2 аргумента".to_string())
                        }
                    }
                    _ => Err(format!("Метод не найден: Response.{}", name)),
                },
                "Request" => match name {
                    "get_params" | "get_param" => {
                        if args.len() == 1 {
                            let param_name = self.evaluate_expr(
                                &args[0],
                                request,
                                response,
                                context,
                                interpreter,
                            )?;
                            Ok(request.get_params(&param_name))
                        } else {
                            Err("Метод Request.get_params требует 1 аргумент".to_string())
                        }
                    },
                    "get_header" => {
                        if args.len() == 1 {
                            let header_name = self.evaluate_expr(&args[0], request, response, context, interpreter)?;
                            Ok(request.headers.get(&header_name).cloned().unwrap_or_default())
                        } else {
                            Err("Метод Request.get_header требует 1 аргумент".to_string())
                        }
                    },
                    "body" | "text_body" => {
                        if args.is_empty() {
                            Ok(request.get_body())
                        } else {
                            Err(format!("Метод Request.{} не принимает аргументы", name))
                        }
                    },
                    "body_base64" => {
                        if args.is_empty() {
                            Ok(request.get_body_as_base64())
                        } else {
                            Err("Метод Request.body_base64 не принимает аргументы".to_string())
                        }
                    },
                    "is_binary" => {
                        if args.is_empty() {
                            Ok(request.is_body_binary().to_string())
                         } else {
                             Err("Метод Request.is_binary не принимает аргументов".to_string())
                         }
                    },
                    _ => Err(format!("Метод не найден: Request.{}", name)),
                },
                _ => unreachable!(),
            }
        } else {
            match name {
                "log_error" => {
                    if args.len() == 1 {
                        let message = self.evaluate_expr(&args[0], request, response, context, interpreter)?;
                        error!("{}", message);
                        Ok("".to_string())
                    } else {
                        Err("Функция log_error требует 1 аргумент".to_string())
                    }
                },
                "log_info" => {
                    if args.len() == 1 {
                        let message = self.evaluate_expr(&args[0], request, response, context, interpreter)?;
                        info!("{}", message);
                        Ok("".to_string())
                    } else {
                        Err("Функция log_info требует 1 аргумент".to_string())
                    }
                },
                "log_trace" => {
                    if args.len() == 1 {
                        let message = self.evaluate_expr(&args[0], request, response, context, interpreter)?;
                        trace!("{}", message);
                        Ok("".to_string())
                    } else {
                        Err("Функция log_trace требует 1 аргумент".to_string())
                    }
                },
                _ => Err(format!("Глобальная функция не найдена: {}", name)),
            }
        }
    }

    fn evaluate_expr(
        &self,
        expr: &RouteAction,
        request: &mut Request,
        response: &mut Response,
        context: &mut ExecutionContext,
        interpreter: &Interpreter,
    ) -> Result<String, String> {
        trace!("Вычисление выражения: {:?}", expr);
        match expr {
            RouteAction::StringLiteral(value) => Ok(value.clone()),
            RouteAction::NumberLiteral(value) => Ok(value.to_string()),
            RouteAction::Identifier(name) => context
                .get_variable(name)
                .cloned()
                .or_else(|| match name.as_str() {
                    "Request" => Some("Request".to_string()),
                    "Response" => Some("Response".to_string()),
                    "Database" => Some("Database".to_string()),
                    _ if interpreter.loaded_plugins.contains_key(name) => Some(name.clone()),
                    _ => None,
                })
                .ok_or_else(|| format!("Переменная, объект или плагин '{}' не найден", name)),
            RouteAction::FunctionCall {
                object,
                name,
                args,
                try_operator: _,
                unwrap_operator: _,
            } => self.execute_function_call(
                object,
                name,
                args,
                request,
                response,
                context,
                interpreter,
            ),
            RouteAction::BinaryOp {
                left,
                operator,
                right,
            } => {
                let left_value =
                    self.evaluate_expr(left, request, response, context, interpreter)?;
                let right_value =
                    self.evaluate_expr(right, request, response, context, interpreter)?;
                trace!(
                    "Бинарная операция: '{}' {} '{}'",
                    left_value, operator, right_value
                );
                match operator.as_str() {
                    "==" => Ok((left_value == right_value).to_string()),
                    "!=" => Ok((left_value != right_value).to_string()),
                    "+" => Ok(format!("{}{}", left_value, right_value)),
                    "+=" => {
                        Err("Оператор '+=' не может быть использован как выражение".to_string())
                    }
                    _ => Err(format!(
                        "Неподдерживаемый бинарный оператор в выражении: {}",
                        operator
                    )),
                }
            }
            RouteAction::PropertyAccess { object, property } => {
                let obj_value_or_name =
                    self.evaluate_expr(object, request, response, context, interpreter)?;
                Err(format!(
                    "Доступ к свойству '{}.{}' не реализован",
                    obj_value_or_name, property
                ))
            }
            RouteAction::VarDeclaration(..) => {
                Err("Объявление переменной не может быть использовано как выражение".to_string())
            }
            RouteAction::Condition { .. } => {
                Err("Условие if не может быть использовано как выражение".to_string())
            }
            RouteAction::Error(msg) => Err(msg.clone()),
        }
    }
}

pub fn handle_request(
    interpreter: &Interpreter,
    method: &str,
    path: &str,
    params: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: HttpBodyVariant,
) -> Response {
    let mut request = Request::new();
    request.params = params;
    request.headers = headers;
    request.body = body;

    let mut response = Response::new();

    for (route_key, (route_path, handler)) in &interpreter.routes {
        let mut local_params = HashMap::new();
        let mut match_found = false;

        if !route_key.starts_with(&format!("{}:", method)) {
            continue;
        }

        if route_path.contains('{') && route_path.contains('}') {
            let route_parts: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();
            let request_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            if route_parts.len() == request_parts.len() {
                let mut current_match = true;
                let mut extracted_params = HashMap::new();
                for (route_part, request_part) in route_parts.iter().zip(request_parts.iter()) {
                    if route_part.starts_with('{') && route_part.ends_with('}') {
                        let param_name = &route_part[1..route_part.len() - 1];
                        extracted_params.insert(param_name.to_string(), request_part.to_string());
                    } else if route_part != request_part {
                        current_match = false;
                        break;
                    }
                }
                if current_match {
                    match_found = true;
                    local_params = extracted_params;
                }
            }
        } else if route_path == path {
            match_found = true;
        }

        if match_found {
            for (k, v) in local_params {
                request.params.insert(k, v);
            }
            handler.execute(&mut request, &mut response, interpreter);
            if !response.headers.contains_key("Content-Type") && response.body.is_some() {
                response.headers.insert(
                    "Content-Type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                );
            }
            return response;
        }
    }

    response.status(404);
    response.body("Not Found").send();
    response.headers.insert(
        "Content-Type".to_string(),
        "text/plain; charset=utf-8".to_string(),
    );
    response
}
