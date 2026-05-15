use log::{debug, trace};
use crate::language::ast::AstNode;
use crate::language::error::Result;
use crate::interpreter_error;
use super::{Interpreter, ErrorHandler};

pub struct Executor {}

impl Executor {
    pub fn new() -> Self {
        Executor {}
    }

    pub fn execute_ast(&self, ast: &AstNode, interpreter: &mut Interpreter) -> Result<()> {
        match ast {
            AstNode::Program(statements) => {
                debug!("Interpreting Program with {} statements", statements.len());

                for stmt in statements {
                    if let AstNode::Import { path, alias } = &**stmt {
                        interpreter.load_plugin(path, alias)?;
                    }
                }

                for stmt in statements {
                    if !matches!(&**stmt, AstNode::Import { .. }) {
                        self.execute_node(stmt, interpreter)?;
                    }
                }

                Ok(())
            },
            AstNode::ServerConfig { routes, tls_config, global_error_handler, config_block } => {
                debug!("Interpreting ServerConfig with {} routes", routes.len());

                if let Some(tls_node) = tls_config {
                    if let AstNode::TlsConfig { enabled, cert_path, key_path } = &**tls_node {
                        interpreter.set_tls_config(*enabled, cert_path.clone(), key_path.clone());
                    } else {
                        return interpreter_error!("Expected TlsConfig node in ServerConfig");
                    }
                }

                if let Some(handler_node) = global_error_handler {
                    if let AstNode::GlobalErrorHandler { error_var, body } = &**handler_node {
                        let actions = self.convert_ast_to_actions(body)?;
                        interpreter.set_global_error_handler(error_var.clone(), actions);
                    } else {
                        return interpreter_error!("Expected GlobalErrorHandler node in ServerConfig");
                    }
                }

                if let Some(config_node) = config_block {
                    if let AstNode::ConfigBlock { config_type, host, port } = &**config_node {
                        interpreter.set_configuration(config_type.clone(), host.clone(), port.clone());
                    } else {
                        return interpreter_error!("Expected ConfigBlock node in ServerConfig");
                    }
                }

                for stmt in routes {
                    if let AstNode::Import { path, alias } = &**stmt {
                        interpreter.load_plugin(path, alias)?;
                    }
                }

                for stmt in routes {
                    if !matches!(&**stmt, AstNode::Import { .. }) {
                        self.execute_node(stmt, interpreter)?;
                    }
                }

                Ok(())
            },
            _ => interpreter_error!(format!("Expected Program or ServerConfig on upper level, found: {:?}", ast)),
        }
    }

    fn execute_node(&self, node: &Box<AstNode>, interpreter: &mut Interpreter) -> Result<()> {
        match &**node {
            AstNode::Route { path, method, body, on_error } => {
                trace!("Interpreting route: {} {}", method, path);

                let actions = self.convert_ast_to_actions(body)?;

                let error_handler = if let Some(on_err) = on_error {
                    match &**on_err {
                        AstNode::ErrorHandlerBlock { error_var, body } => {
                            let err_actions = self.convert_ast_to_actions(body)?;
                            Some(ErrorHandler {
                                error_var: error_var.clone(),
                                actions: err_actions,
                            })
                        },
                        _ => return interpreter_error!("Expected ErrorHandlerBlock for route"),
                    }
                } else {
                    None
                };

                let route_handler = super::route_handler::RouteHandler::new(actions, error_handler);
                interpreter.add_route(path.clone(), method.clone(), route_handler);
                Ok(())
            },
            AstNode::Import { .. } => Ok(()),
            _ => interpreter_error!(format!("Unexpected type of node in main loop of execution: {:?}", node)),
        }
    }

    fn convert_ast_to_actions(&self, node: &AstNode) -> Result<Vec<Box<AstNode>>> {
        match node {
            AstNode::Block(statements) => Ok(statements.clone()),
            _ => {
                let mut actions = Vec::new();
                actions.push(Box::new(node.clone()));
                Ok(actions)
            }
        }
    }
}