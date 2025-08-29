mod context;
mod evaluator;
mod executor;
mod route_handler;
pub mod builtin;

use std::collections::HashMap;
use std::path::Path;
use log::{debug, info, warn};
use crate::language::ast::AstNode;
use crate::language::error::Result;
use crate::interpreter_error;
use crate::servers::http_core::TlsConfig;
use executor::Executor;
use route_handler::RouteHandler;
use builtin::plugin::PluginManager;
use builtin::response::Response;
use builtin::request::Request;
use builtin::request::HttpBodyVariant;

#[derive(Debug, Clone)]
pub struct Configuration {
    pub config_type: String,
    pub host: String,
    pub port: String,
}

#[derive(Debug, Clone)]
pub struct ErrorHandler {
    pub error_var: String,
    pub actions: Vec<Box<AstNode>>,
}

#[derive(Debug)]
pub struct Interpreter {
    pub routes: HashMap<String, (String, RouteHandler)>,
    pub tls_config: Option<TlsConfig>,
    pub global_error_handler: Option<ErrorHandler>,
    pub configuration: Option<Configuration>,
    pub plugin_manager: PluginManager,
}

impl Interpreter {
    pub fn new() -> Self {
        dbg!("Starting interpreter");
        Interpreter {
            routes: HashMap::new(),
            tls_config: None,
            global_error_handler: None,
            configuration: None,
            plugin_manager: PluginManager::new(),
        }
    }

    pub fn interpret(&mut self, ast: &AstNode) -> Result<()> {
        debug!("Начало интерпретации AST узла: {:?}", ast);

        let executor = Executor::new();
        executor.execute_ast(ast, self)?;

        info!("Интерпретация конфигурации успешно завершена.");
        Ok(())
    }

    pub fn handle_request(
        &self,
        method: &str,
        path: &str,
        params: HashMap<String, String>,
        headers: HashMap<String, String>,
        body: HttpBodyVariant,
    ) -> Response {
        let mut request = Request::new(params, headers, body);
        let mut response = Response::new();

        for (route_key, (route_path, handler)) in &self.routes {
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
                handler.execute(&mut request, &mut response, &self.plugin_manager, self.global_error_handler.as_ref());
                return response;
            }
        }

        response.status(404);
        response.body("Not Found");
        response.send();
        response
    }

    pub fn add_route(&mut self, path: String, method: String, handler: RouteHandler) {
        let route_key = format!("{}:{}", method, path);
        if self.routes.contains_key(&route_key) {
            warn!("Переопределение маршрута: {}", route_key);
        }
        debug!("Добавление обработчика для маршрута: {}", route_key);
        self.routes.insert(route_key, (path, handler));
    }

    pub fn set_tls_config(&mut self, enabled: bool, cert_path: String, key_path: String) {
        self.tls_config = Some(TlsConfig {
            enabled,
            cert_path,
            key_path,
        });
        debug!("Конфигурация TLS установлена: enabled={}", enabled);
    }

    pub fn set_global_error_handler(&mut self, error_var: String, actions: Vec<Box<AstNode>>) {
        self.global_error_handler = Some(ErrorHandler {
            error_var: error_var.clone(),
            actions,
        });
        debug!("Глобальный обработчик ошибок установлен для переменной '{}'", error_var);
    }

    pub fn set_configuration(&mut self, config_type: String, host: String, port: String) {
        self.configuration = Some(Configuration {
            config_type: config_type.clone(),
            host: host.clone(),
            port: port.clone(),
        });
        debug!("Конфигурация сервера установлена: type={}, host={}, port={}", config_type, host, port);
    }

    pub fn load_plugin(&mut self, path: &str, alias: &str) -> Result<()> {
        debug!("Загрузка плагина: '{}' из '{}'", alias, path);

        if !Path::new(path).exists() {
            return interpreter_error!(format!("Плагин не найден по пути: {}", path));
        }

        self.plugin_manager.load_plugin(path, alias)
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
            plugin_manager: PluginManager::new(),
        }
    }
}