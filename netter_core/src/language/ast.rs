use std::fmt;

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
    ArrayLiteral(Vec<Box<AstNode>>),        // [1, "hey", 3]
    ArrayAccess {                           // arr[0]
        array: Box<AstNode>,
        index: Box<AstNode>,
    },
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
    },
    ForLoop {
        var_name: String,
        iterable: Box<AstNode>,
        body: Box<AstNode>,
    },
    WhileLoop {
        condition: Box<AstNode>,
        body: Box<AstNode>,
    }
}

pub trait AstVisitor<T> {
    type Error;

    fn visit_program(&mut self, statements: &[Box<AstNode>]) -> Result<T, Self::Error>;
    fn visit_route(&mut self, path: &str, method: &str, body: &AstNode, on_error: Option<&AstNode>) -> Result<T, Self::Error>;
    fn visit_block(&mut self, statements: &[Box<AstNode>]) -> Result<T, Self::Error>;
    fn visit_var_declaration(&mut self, name: &str, value: &AstNode) -> Result<T, Self::Error>;
    fn visit_function_call(&mut self, object: Option<&AstNode>, name: &str, args: &[Box<AstNode>], try_op: bool, unwrap_op: bool) -> Result<T, Self::Error>;
    fn visit_property_access(&mut self, object: &AstNode, property: &str) -> Result<T, Self::Error>;
    fn visit_if_statement(&mut self, condition: &AstNode, then_branch: &AstNode, else_branch: Option<&AstNode>) -> Result<T, Self::Error>;
    fn visit_string_literal(&mut self, value: &str) -> Result<T, Self::Error>;
    fn visit_number_literal(&mut self, value: i64) -> Result<T, Self::Error>;
    fn visit_identifier(&mut self, name: &str) -> Result<T, Self::Error>;
    fn visit_binary_op(&mut self, left: &AstNode, operator: &str, right: &AstNode) -> Result<T, Self::Error>;
    fn visit_tls_config(&mut self, enabled: bool, cert_path: &str, key_path: &str) -> Result<T, Self::Error>;
    fn visit_server_config(&mut self, routes: &[Box<AstNode>], tls_config: Option<&AstNode>, global_error_handler: Option<&AstNode>, config_block: Option<&AstNode>) -> Result<T, Self::Error>;
    fn visit_global_error_handler(&mut self, error_var: &str, body: &AstNode) -> Result<T, Self::Error>;
    fn visit_error_handler_block(&mut self, error_var: &str, body: &AstNode) -> Result<T, Self::Error>;
    fn visit_config_block(&mut self, config_type: &str, host: &str, port: &str) -> Result<T, Self::Error>;
    fn visit_import(&mut self, path: &str, alias: &str) -> Result<T, Self::Error>;
    fn visit_while_loop(&mut self, condition: &AstNode, body: &AstNode) -> Result<T, Self::Error>;
    fn visit_for_loop(&mut self, var_name: &str, iterable: &AstNode, body: &AstNode) -> Result<T, Self::Error>;
    fn visit_array_literal(&mut self, values: &[Box<AstNode>]) -> Result<T, Self::Error>;
    fn visit_array_access(&mut self, array: &AstNode, index: &AstNode) -> Result<T, Self::Error>;
}

impl AstNode {
    pub fn accept<T, V: AstVisitor<T>>(&self, visitor: &mut V) -> Result<T, V::Error> {
        match self {
            AstNode::Program(statements) => visitor.visit_program(statements),
            AstNode::Route { path, method, body, on_error } =>
                visitor.visit_route(path, method, body, on_error.as_ref().map(|b| b.as_ref())),
            AstNode::Block(statements) => visitor.visit_block(statements),
            AstNode::VarDeclaration { name, value } => visitor.visit_var_declaration(name, value),
            AstNode::FunctionCall { object, name, args, try_operator, unwrap_operator } =>
                visitor.visit_function_call(object.as_ref().map(|o| o.as_ref()), name, args, *try_operator, *unwrap_operator),
            AstNode::PropertyAccess { object, property } => visitor.visit_property_access(object, property),
            AstNode::IfStatement { condition, then_branch, else_branch } =>
                visitor.visit_if_statement(condition, then_branch, else_branch.as_ref().map(|b| b.as_ref())),
            AstNode::StringLiteral(value) => visitor.visit_string_literal(value),
            AstNode::NumberLiteral(value) => visitor.visit_number_literal(*value),
            AstNode::Identifier(name) => visitor.visit_identifier(name),
            AstNode::BinaryOp { left, operator, right } => visitor.visit_binary_op(left, operator, right),
            AstNode::TlsConfig { enabled, cert_path, key_path } =>
                visitor.visit_tls_config(*enabled, cert_path, key_path),
            AstNode::ServerConfig { routes, tls_config, global_error_handler, config_block } =>
                visitor.visit_server_config(
                    routes,
                    tls_config.as_ref().map(|t| t.as_ref()),
                    global_error_handler.as_ref().map(|g| g.as_ref()),
                    config_block.as_ref().map(|c| c.as_ref())
                ),
            AstNode::GlobalErrorHandler { error_var, body } => visitor.visit_global_error_handler(error_var, body),
            AstNode::ErrorHandlerBlock { error_var, body } => visitor.visit_error_handler_block(error_var, body),
            AstNode::ConfigBlock { config_type, host, port } => visitor.visit_config_block(config_type, host, port),
            AstNode::Import { path, alias } => visitor.visit_import(path, alias),
            AstNode::WhileLoop { condition, body } => visitor.visit_while_loop(condition, body),
            AstNode::ForLoop { var_name, iterable, body } => visitor.visit_for_loop(var_name, iterable, body),
            AstNode::ArrayLiteral(elements) => visitor.visit_array_literal(elements),
            AstNode::ArrayAccess { array, index } => visitor.visit_array_access(array, index),
        }
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
            AstNode::ArrayLiteral(value) => write!(f, "[{:?}]", value),
            AstNode::ArrayAccess { array, index } => {
                write!(f, "{}[{}]", array, index)
            },
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
            },
            AstNode::WhileLoop { condition, body } => {
                writeln!(f, "while {}", condition)
            },
            AstNode::ForLoop {var_name, iterable, body} => {
                writeln!(f, "for {} in {}", var_name, iterable)
            },
        }
    }
}