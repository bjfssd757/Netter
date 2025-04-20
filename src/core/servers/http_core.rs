use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{body::{self, Bytes}, header::{HeaderName, HeaderValue}, server::conn::http1, service::service_fn, Request, Response, StatusCode};
use log::{debug, trace, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use derive_more::Debug;
use std::fs::File;
use std::io::BufReader;
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use hyper_util::rt::TokioIo;
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
    tls_config: Option<TlsConfig>,
    rustls_config: Option<Arc<ServerConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub(crate) enabled: bool,
    pub(crate) cert_path: String,
    pub(crate) key_path: String,
}

#[allow(dead_code)]
pub trait HTTP {
    fn new(addr: Vec<u16>, port: u16, routes: Routes) -> Self;
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

// #[derive(Serialize, Deserialize)]
// pub struct ConfigHttp {
//     host: String,
//     port: u16,
//     routes: Vec<RouteConfig>,
//     tls: Option<TlsConfig>,
// }

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

        let tls_config = config.tls;
        let rustls_config = if let Some(tls) = &tls_config {
            if tls.enabled {
                match load_rustls_config(&tls.cert_path, &tls.key_path) {
                    Ok(config) => Some(Arc::new(config)),
                    Err(e) => {
                        error!("Failed to load TLS configuration: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Server {
            addr: config.host.split('.')
                .map(|s| s.parse::<u16>().unwrap_or(127))
                .collect(),
            port: config.port,
            routes,
            interpreter: None,
            tls_config,
            rustls_config,
        })
    }

    pub fn from_interpreter(
        addr: Vec<u16>, 
        port: u16, 
        interpreter: Interpreter,
        tls_config: Option<TlsConfig>,
    ) -> Self {
        let rustls_config = if let Some(tls) = &tls_config {
            if tls.enabled {
                match load_rustls_config(&tls.cert_path, &tls.key_path) {
                    Ok(config) => Some(Arc::new(config)),
                    Err(e) => {
                        error!("Failed to load TLS configuration: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        Server {
            addr,
            port,
            routes: Vec::new(),
            interpreter: Some(Arc::new(Mutex::new(interpreter))),
            tls_config,
            rustls_config,
        }
    }

    pub fn from_ast(
        addr: Vec<u16>, 
        port: u16, 
        ast: &AstNode,
        tls_config: Option<TlsConfig>,
    ) -> Result<Self, String> {
        let mut interpreter = Interpreter::new();
        interpreter.interpret(ast)?;
        
        Ok(Server::from_interpreter(addr, port, interpreter, tls_config))
    }

    pub fn enable_tls(&mut self, cert_path: String, key_path: String) -> Result<(), Box<dyn std::error::Error>> {
        let tls_config = TlsConfig {
            enabled: true,
            cert_path: cert_path.clone(),
            key_path: key_path.clone(),
        };

        let rustls_config = load_rustls_config(&cert_path, &key_path)?;
        
        self.tls_config = Some(tls_config);
        self.rustls_config = Some(Arc::new(rustls_config));
        
        Ok(())
    }

    pub fn disable_tls(&mut self) {
        if let Some(tls_config) = &mut self.tls_config {
            tls_config.enabled = false;
        }
        self.rustls_config = None;
    }

    pub fn is_tls_enabled(&self) -> bool {
        self.tls_config.as_ref().map_or(false, |c| c.enabled) && self.rustls_config.is_some()
    }
}

fn load_rustls_config(cert_path: &str, key_path: &str) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let cert_file = File::open(cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);

    let mut cert_chain = Vec::new();
    for cert_result in certs(&mut cert_reader) {
        let cert = cert_result?;
        cert_chain.push(cert.to_owned());
    }

    if cert_chain.is_empty() {
        return Err("No certificates found".into());
    }

    let key_file = File::open(key_path)?;
    let mut key_reader = BufReader::new(key_file);

    let mut keys = Vec::new();
    for key_result in pkcs8_private_keys(&mut key_reader) {
        let key = key_result?;
        keys.push(key.into());
    }
    
    if keys.is_empty() {
        return Err("No private keys found".into());
    }
    
    let private_key = keys.remove(0);

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)?;
    
    Ok(config)
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
            tls_config: None,
            rustls_config: None,
        }
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr_str = self.addr.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".");
        let addr = format!("{}:{}", addr_str, self.port);
        
        let protocol = if self.is_tls_enabled() { "HTTPS" } else { "HTTP" };
        state::save_state(String::from(protocol), addr_str.clone(), self.port)
            .map_err(|_| "Failed to save state")?;

        let server_state = Arc::new(Mutex::new(self.clone()));
        state::load_state();

        let listener = TcpListener::bind(&addr).await?;
        
        info!("Server starting at {}", addr);

        if self.is_tls_enabled() {
            info!("Starting HTTPS server on {}", addr);
            
            let tls_config = self.rustls_config.clone()
                .ok_or("TLS is enabled but configuration is missing")?;
                
            let tls_acceptor = TlsAcceptor::from(tls_config);
            trace!("HTTPS server started on {}", addr);

            loop {
                let (tcp_stream, _) = listener.accept().await?;
                let acceptor = tls_acceptor.clone();
                let server_clone = server_state.clone();
                
                tokio::task::spawn(async move {
                    match acceptor.accept(tcp_stream).await {
                        Ok(tls_stream) => {
                            let io = TokioIo::new(tls_stream);
                            
                            if let Err(err) = http1::Builder::new()
                                .serve_connection(io, service_fn(move |req| {
                                    let server_clone = server_clone.clone();
                                    async move { handler(req, server_clone).await }
                                }))
                                .await
                            {
                                error!("Error serving TLS connection: {}", err);
                            }
                        },
                        Err(err) => {
                            error!("TLS handshake failed: {}", err);
                        }
                    }
                });
            }
        } else {
            info!("Starting HTTP server on {}", addr);

            loop {
                let (tcp_stream, _) = listener.accept().await?;
                let io = TokioIo::new(tcp_stream);
                
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
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Server {
            addr: self.addr.clone(),
            port: self.port,
            routes: self.routes.clone(),
            interpreter: self.interpreter.clone(),
            tls_config: self.tls_config.clone(),
            rustls_config: self.rustls_config.clone(),
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