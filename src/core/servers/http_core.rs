use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{body::{self, Bytes}, header::{HeaderName, HeaderValue}, server::conn::http1, service::service_fn, Request, Response, StatusCode};
use log::{debug, trace, error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use derive_more::Debug;
use crate::{core::{config_parser::load_config, language::{interpreter::Interpreter, lexer::AstNode}}, state};

#[derive(Debug)]
pub struct Routes {
    method: String,
    path: String,
    response: Resp,
}

#[derive(Debug)]
pub struct Resp {
    body: String,
    headers: Vec<(String, String)>,
    status: u16,
}

#[derive(Debug)]
pub struct Server {
    addr: Vec<u16>,
    port: u16,
    routes: Vec<Routes>,
    #[debug(skip)]
    interpreter: Option<Arc<Mutex<Interpreter>>>,
}

#[allow(dead_code)]
pub trait HTTP {
    fn new(addr: Vec<u16>, port: u16, routes: Routes) -> Self;
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    host: String,
    port: u16,
    routes: Vec<RouteConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct RouteConfig {
    method: String,
    path: String,
    response: RespConfig,
}

#[derive(Serialize, Deserialize)]
pub struct RespConfig {
    body: String,
    headers: Vec<(String, String)>,
    status: u16,
}

impl Server {
    pub fn configure(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        debug!("Read config data");

        let config = load_config(file_path)?;

        let routes = config.routes.into_iter().map(|route| Routes {
            method: route.method,
            path: route.path,
            response: Resp {
                body: route.response.body,
                headers: route.response.headers,
                status: route.response.status,
            },
        }).collect();

        Ok(Server {
            addr: config.host.split('.')
                .map(|s| s.parse::<u16>().unwrap_or(127))
                .collect(),
            port: config.port,
            routes,
            interpreter: None,
        })
    }

    pub fn from_interpreter(
        addr: Vec<u16>, 
        port: u16, 
        interpreter: Interpreter
    ) -> Self {
        Server {
            addr,
            port,
            routes: Vec::new(),
            interpreter: Some(Arc::new(Mutex::new(interpreter))),
        }
    }

    pub fn from_ast(
        addr: Vec<u16>, 
        port: u16, 
        ast: &AstNode
    ) -> Result<Self, String> {
        let mut interpreter = Interpreter::new();
        interpreter.interpret(ast)?;
        
        Ok(Server::from_interpreter(addr, port, interpreter))
    }
}

async fn handler(
    req: Request<body::Incoming>,
    server_state: Arc<Mutex<Server>>
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let server = server_state.lock().unwrap();
    
    if let Some(interpreter) = &server.interpreter {
        let method = req.method().as_str().to_string();
        let path = req.uri().path().to_string();
        
        let params = HashMap::new();
        
        let response_obj = crate::core::language::interpreter::handle_request(
            &interpreter.lock().unwrap(),
            &method,
            &path,
            params
        );
        
        let mut response = Response::new(full(response_obj.body.unwrap_or_default()));
        *response.status_mut() = StatusCode::from_u16(response_obj.status).unwrap_or(StatusCode::OK);
        
        for (key, value) in &response_obj.headers {
            if let Ok(header_name) = HeaderName::from_str(key) {
                if let Ok(header_value) = HeaderValue::from_str(value) {
                    response.headers_mut().insert(header_name, header_value);
                }
            }
        }
        
        return Ok(response);
    }
    
    for route in &server.routes {
        if req.method().as_str() == route.method && req.uri().path() == route.path {
            let mut response = Response::new(full(route.response.body.clone()));
            *response.status_mut() = StatusCode::from_u16(route.response.status).unwrap_or(StatusCode::OK);
            for (key, value) in &route.response.headers {
                if let Ok(header_name) = HeaderName::from_str(key) {
                    if let Ok(header_value) = HeaderValue::from_str(value) {
                        response.headers_mut().insert(header_name, header_value);
                    }
                }
            }
            return Ok(response);
        }
    }

    let mut not_found = Response::new(empty());
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}

impl HTTP for Server {
    fn new(addr: Vec<u16>, port: u16, routes: Routes) -> Self {
        Server {
            addr,
            port,
            routes: vec![routes],
            interpreter: None,
        }
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!(
            "{}:{}",
            self.addr.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("."),
            self.port
        );
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        trace!("Server started on {}", addr);

        state::save_state(String::from("HTTP"), 
            self.addr.clone().iter().map(|n| n.to_string()).collect::<Vec<_>>().join("."), 
            self.port)
            .map_err(|_| "Failed to save state")?;

        let server_state = Arc::new(Mutex::new(self.clone()));
        state::load_state();

        loop {
            let (socket, _) = listener.accept().await?;
            let io = hyper_util::rt::TokioIo::new(socket);
            
            let server_clone = server_state.clone();
            
            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(move |req| {
                        let server_clone = server_clone.clone();
                        async move { handler(req, server_clone).await }
                    }))
                    .await
                {
                    error!("Error serving connection: {}", err);
                }
            });
        }
    }
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Server {
            addr: self.addr.clone(),
            port: self.port,
            routes: self.routes.clone(),
            interpreter: self.interpreter.clone(),
        }
    }
}

impl Clone for Routes {
    fn clone(&self) -> Self {
        Routes {
            method: self.method.clone(),
            path: self.path.clone(),
            response: self.response.clone(),
        }
    }
}

impl Clone for Resp {
    fn clone(&self) -> Self {
        Resp {
            body: self.body.clone(),
            headers: self.headers.clone(),
            status: self.status,
        }
    }
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}