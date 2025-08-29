use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{
    body::{self, Bytes},
    header::{HeaderName, HeaderValue, CONTENT_TYPE},
    Request,
    Response,
    StatusCode,
};
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, fs::File, io::BufReader, str::FromStr, sync::{Arc, Mutex}};
use derive_more::Debug;
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use hyper::service::service_fn;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as HyperAutoBuilder,
};
use tokio_rustls::TlsAcceptor;
use tokio::net::TcpListener;
use crate::{
    language::interpreter::Interpreter,
    CoreError,
};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug, Clone)] 
pub struct Server {
    #[debug(skip)] 
    pub interpreter: Option<Arc<Mutex<Interpreter>>>,
    pub tls_config: Option<TlsConfig>, 
    pub rustls_config: Option<Arc<ServerConfig>>, 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone)]
pub enum HttpBodyVariant {
    Text(String),
    Bytes(Vec<u8>),
    Empty,
}

impl Default for HttpBodyVariant {
    fn default() -> Self {
        HttpBodyVariant::Empty
    }
}

impl Server {
    pub fn from_interpreter(
        interpreter: Interpreter,
        tls_config: Option<TlsConfig>, 
    ) -> Self {
        let rustls_config_result = if let Some(tls) = &tls_config {
            if tls.enabled {
                info!("Loading rustls config: cert='{}', key='{}'", tls.cert_path, tls.key_path);
                match load_rustls_config(&tls.cert_path, &tls.key_path) {
                    Ok(config) => {
                        info!("Rustls config loaded successfully.");
                        Some(Arc::new(config))
                    },
                    Err(e) => {
                        error!("Failed to load rustls configuration: {:?}", e);
                        None 
                    }
                }
            } else {
                info!("TLS is configured but disabled.");
                None 
            }
        } else {
            info!("No TLS configuration provided.");
            None 
        };

        Server {
            interpreter: Some(Arc::new(Mutex::new(interpreter))),
            tls_config,
            rustls_config: rustls_config_result,
        }
    }

    pub fn enable_tls(&mut self, cert_path: String, key_path: String) -> Result<(), CoreError> {
        info!("Enabling TLS: cert='{}', key='{}'", cert_path, key_path);
        let tls_config = TlsConfig {
            enabled: true,
            cert_path: cert_path.clone(),
            key_path: key_path.clone(),
        };
        let rustls_config = load_rustls_config(&cert_path, &key_path)?; 
        self.tls_config = Some(tls_config);
        self.rustls_config = Some(Arc::new(rustls_config));
        info!("TLS enabled and rustls config loaded.");
        Ok(())
    }

    pub fn disable_tls(&mut self) {
        info!("Disabling TLS.");
        if let Some(tls_config) = &mut self.tls_config {
            tls_config.enabled = false;
        }
        self.rustls_config = None;
    }
    pub fn is_tls_enabled(&self) -> bool {
        self.tls_config.as_ref().map_or(false, |c| c.enabled) && self.rustls_config.is_some()
    }
}

fn load_rustls_config(cert_path: &str, key_path: &str) -> Result<ServerConfig, CoreError> {
    debug!("Loading cert file from: {}", cert_path);
    let cert_file = File::open(cert_path)
        .map_err(|e| CoreError::IoError(format!("Failed to open cert file '{}': {}", cert_path, e)))?;
    let mut cert_reader = BufReader::new(cert_file);

    let cert_chain = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>() 
        .map_err(|e| CoreError::IoError(format!("Failed to read certificates from '{}': {}", cert_path, e)))?;

    if cert_chain.is_empty() {
         error!("No valid certificates found in file: {}", cert_path);
        return Err(CoreError::ConfigParseError(format!("No certificates found in '{}'", cert_path)));
    }
    debug!("Found {} certificate(s) in {}", cert_chain.len(), cert_path);

    debug!("Loading private key file from: {}", key_path);
    let key_file = File::open(key_path)
        .map_err(|e| CoreError::IoError(format!("Failed to open key file '{}': {}", key_path, e)))?;
    let mut key_reader = BufReader::new(key_file);
    let private_key = pkcs8_private_keys(&mut key_reader)
        .next() 
        .ok_or_else(|| CoreError::ConfigParseError(format!("No PKCS8 private keys found in '{}'", key_path)))? 
        .map_err(|e| CoreError::IoError(format!("Failed to read private key from '{}': {}", key_path, e)))?; 

    debug!("Private key loaded successfully from {}", key_path);
    
    let config = ServerConfig::builder()
        .with_no_client_auth() 
        .with_single_cert(cert_chain, private_key.into()) 
        .map_err(|e| CoreError::ConfigParseError(format!("Failed to build rustls ServerConfig: {}", e)))?;

    Ok(config)
}

pub async fn handle_http_request(
    req: Request<body::Incoming>,
    server_state: Arc<Server>, 
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Infallible> { 
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let hyper_headers = req.headers().clone();
    let start_time = std::time::Instant::now(); 

    trace!("[Req: {} {}] Handling request.", method, path);

    let query_params: HashMap<String, String> = req.uri().query()
        .map(|v| url::form_urlencoded::parse(v.as_bytes()).into_owned().collect())
        .unwrap_or_else(HashMap::new);
    trace!("[Req: {} {}] Query params: {:?}", method, path, query_params);

    let body_bytes_result = req.into_body().collect().await; 

    let interpretable_body: HttpBodyVariant = match body_bytes_result {
        Ok(collected_body) => {
            let actual_body_bytes = collected_body.to_bytes();
            if actual_body_bytes.is_empty() {
                HttpBodyVariant::Empty
            } else {
                let content_type_header = hyper_headers.get(CONTENT_TYPE);
                let is_likely_file_upload = content_type_header
                    .and_then(|val| val.to_str().ok())
                    .map_or(false, |ct| {
                        ct.starts_with("multipart/form-data") ||
                        ct.starts_with("application/octet-stream")
                    });

                if is_likely_file_upload {
                    trace!("[Req: {} {}] Detected file upload (Content-Type: {:?}). Transfering raw bytes...", method, path, content_type_header.map(|h|h.to_str()));
                    HttpBodyVariant::Bytes(actual_body_bytes.to_vec())
                } else {
                    match String::from_utf8(actual_body_bytes.to_vec()) {
                        Ok(s) => {
                            trace!("[Req: {} {}] Тело (UTF-8 текст): {}", method, path, s);
                            HttpBodyVariant::Text(s)
                        },
                        Err(e) => {
                            warn!("[Req: {} {}] Failed to decode body as UTF-8 (Content-Type: {:?}): {}. Transfering raw bytes...",
                                method,
                                path,
                                content_type_header.map(|h|h.to_str()), e);
                            HttpBodyVariant::Bytes(actual_body_bytes.to_vec())
                        }
                    }
                }
            }
        },
        Err(e) => {
            error!("[Req: {} {}] Failed to collect request body: {}", method, path, e);
            let mut response = hyper::Response::new(full("Failed while reading request body."));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            return Ok(response);
        }
    };

    let mut header_map = HashMap::new();
    for (key, value) in &hyper_headers {
        match value.to_str() {
            Ok(value_string) => {
                header_map.insert(key.as_str().to_string(), value_string.to_string());
            },
            Err(_) => {
                warn!("[Req: {} {}] Header '{}' contains non-UTF8 value, skipping.", method, path, key.as_str());
            }
        }
    }
    trace!("[Req: {} {}] Headers: {:?}", method, path, header_map);

    let response_result: Result<Response<BoxBody<Bytes, hyper::Error>>, Infallible> = {
        if let Some(interpreter_arc) = &server_state.interpreter {
            let interpreter_guard = match interpreter_arc.lock() {
                 Ok(guard) => guard,
                 Err(poisoned) => {
                     error!("[Req: {} {}] Interpreter mutex is poisoned! {}", method, path, poisoned);
                     let mut response = hyper::Response::new(full("Internal Server Error (Mutex Poisoned)"));
                     *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                     return Ok(response); 
                 }
            };
            trace!("[Req: {} {}] Interpreter locked.", method, path);

            let body = match interpretable_body {
                HttpBodyVariant::Bytes(bytes) => {
                    crate::language::interpreter::builtin::request::HttpBodyVariant::Bytes(bytes)
                },
                HttpBodyVariant::Text(text) => {
                    crate::language::interpreter::builtin::request::HttpBodyVariant::Text(text)
                },
                HttpBodyVariant::Empty => {
                    crate::language::interpreter::builtin::request::HttpBodyVariant::Empty
                },
            };
            let response_obj = Interpreter::handle_request(
                &interpreter_guard,
                method.as_str(),
                &path,
                query_params,
                header_map,
                body,
            );
            trace!("[Req: {} {}] Interpreter response object: {:?}", method, path, response_obj);
            let mut response = Response::new(full(response_obj.body.unwrap_or_default())); 

            *response.status_mut() = StatusCode::from_u16(response_obj.status)
                .unwrap_or_else(|_| {
                    warn!("[Req: {} {}] Invalid status code from interpreter: {}", method, path, response_obj.status);
                    StatusCode::INTERNAL_SERVER_ERROR 
                });
            
            for (key, value) in &response_obj.headers {
                match HeaderName::from_str(key) {
                    Ok(header_name) => {
                        match HeaderValue::from_str(value) {
                            Ok(header_value) => {
                                response.headers_mut().insert(header_name, header_value);
                            },
                            Err(_) => {
                                warn!("[Req: {} {}] Invalid header value for key '{}': {}", method, path, key, value);
                            }
                        }
                    },
                    Err(_) => {
                         warn!("[Req: {} {}] Invalid header name: {}", method, path, key);
                    }
                }
            }
            trace!("[Req: {} {}] Response headers set: {:?}", method, path, response.headers());

            if !response.headers().contains_key(hyper::header::CONTENT_TYPE) {
                 response.headers_mut().insert(hyper::header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"));
            }
             if !response.headers().contains_key(hyper::header::SERVER) {
                 response.headers_mut().insert(hyper::header::SERVER, HeaderValue::from_static("Netter/HTTP"));
            }

            Ok(response) 
        } else {
            warn!("[Req: {} {}] HTTP request received but no interpreter is configured for this server instance.", method, path);
            let mut not_found = Response::new(full("Not Found (Interpreter Not Configured)"));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found) 
        }
    };

    
    match &response_result {
        Ok(resp) => {
            info!(
                "[Req: {} {}] Responded with status {} in {:?}",
                method,
                path,
                resp.status(),
                start_time.elapsed()
            );
        }
        Err(_) => {
             error!(
                "[Req: {} {}] Handler resulted in an unexpected error after {:?}",
                method,
                path,
                start_time.elapsed()
            );
        }
    }

    response_result
}

#[allow(dead_code)]
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

pub async fn run_hyper_server(
    socket_addr_str: String,
    server_state: Arc<Server>,
    server_id: String,
) {
    info!(
        "[HTTP Server ID: {}] Attempting to bind on {}...",
        server_id, socket_addr_str
    );

    let socket_addr = match SocketAddr::from_str(&socket_addr_str) {
        Ok(addr) => addr,
        Err(e) => {
            error!(
                "[HTTP Server ID: {}] Invalid address '{}': {}",
                server_id, socket_addr_str, e
            );
            return;
        }
    };

    let listener = match TcpListener::bind(socket_addr).await {
        Ok(l) => {
            info!(
                "[HTTP Server ID: {}] Listening on {}",
                server_id, socket_addr
            );
            l
        }
        Err(e) => {
            error!(
                "[HTTP Server ID: {}] Bind failed {}: {}",
                server_id, socket_addr, e
            );
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((tcp_stream, remote_addr)) => {
                trace!(
                    "[HTTP Server ID: {}] Accepted from {}",
                    server_id,
                    remote_addr
                );
                let state_for_service = server_state.clone();
                let state_for_tls_check = server_state.clone();
                let id_clone = server_id.clone();

                tokio::spawn(async move {
                    let executor = TokioExecutor::new();

                    let hyper_service = service_fn(move |req| {
                        handle_http_request(req, state_for_service.clone())
                    });

                    if state_for_tls_check.is_tls_enabled() {
                        if let Some(tls_conf) = state_for_tls_check.rustls_config.clone() {
                            let acceptor = TlsAcceptor::from(tls_conf);
                            match acceptor.accept(tcp_stream).await {
                                Ok(tls_stream) => {
                                    trace!(
                                        "[HTTP Server ID: {}] TLS Handshake OK for {}",
                                        id_clone,
                                        remote_addr
                                    );
                                    let io = TokioIo::new(tls_stream);
                                    if let Err(err) = HyperAutoBuilder::new(executor)
                                        .serve_connection(io, hyper_service)
                                        .await
                                    {
                                        let is_incomplete = err
                                            .downcast_ref::<hyper::Error>()
                                            .map_or(false, |he| he.is_incomplete_message());
                                        if !is_incomplete {
                                            error!(
                                                "[HTTP Server ID: {}] TLS Conn error {}: {}",
                                                id_clone, remote_addr, err
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "[HTTP Server ID: {}] TLS Handshake error {}: {}",
                                        id_clone, remote_addr, e
                                    );
                                }
                            }
                        } else {
                            error!(
                                "[HTTP Server ID: {}] TLS enabled but config missing!",
                                id_clone
                            );
                        }
                    } else {
                        let io = TokioIo::new(tcp_stream);
                        if let Err(err) = HyperAutoBuilder::new(executor)
                            .serve_connection(io, hyper_service)
                            .await
                        {
                            let is_incomplete = err
                                .downcast_ref::<hyper::Error>()
                                .map_or(false, |he| he.is_incomplete_message());
                            if !is_incomplete {
                                error!(
                                    "[HTTP Server ID: {}] HTTP Conn error {}: {}",
                                    id_clone, remote_addr, err
                                );
                            }
                        }
                    }
                    trace!(
                        "[HTTP Server ID: {}] Conn finished for {}",
                        id_clone,
                        remote_addr
                    );
                });
            }
            Err(e) => {
                error!(
                    "[HTTP Server ID: {}] Accept error: {}. Pausing...",
                    server_id, e
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}