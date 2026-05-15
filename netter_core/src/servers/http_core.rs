use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex}, time::Duration};
use axum::{Router, body::Body, extract::{Request, State}, response::IntoResponse, routing::any};
use axum_server::Handle;
use http_body_util::BodyExt;
use hyper::{HeaderMap, StatusCode, header::CONTENT_LENGTH};
use log::{error, warn, info};
use rustls::ServerConfig;
use tokio::sync::mpsc;
use derive_more::Debug;
use super::TlsConfig;
use crate::{CoreError, language::{Interpreter, interpreter::builtin::request::HttpBodyVariant}, servers::{Server, load_rustls_config}};

#[derive(Debug, Clone, Copy)]
pub enum ServerCommand {
    Restart,
    Stop,
}

#[derive(Debug, Clone)] 
pub struct HttpServer {
    #[debug(skip)] 
    pub interpreter: Option<Arc<Mutex<Interpreter>>>,
    pub tls_config: Option<TlsConfig>, 
    pub rustls_config: Option<Arc<ServerConfig>>, 
    pub server_id: String,
    addr: Option<SocketAddr>,
    server_handle: Option<Handle<SocketAddr>>,
    control_tx: Option<mpsc::Sender<ServerCommand>>,
    boot_time: Option<std::time::Instant>,
}

impl HttpServer {
    pub fn from_interpreter(
        interpreter: Interpreter,
        tls_config: Option<TlsConfig>,
        server_id: String,
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

        Self {
            interpreter: Some(Arc::new(Mutex::new(interpreter))),
            tls_config,
            rustls_config: rustls_config_result,
            server_id,
            addr: None,
            control_tx: None,
            server_handle: None,
            boot_time: None,
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

impl Server for HttpServer {
    /// Start HTTP server on given address
    /// 
    /// # Example
    /// 
    /// ```rust
    /// let mut server = HttpServer::from_interpreter(interpreter, None, "1234".to_string());
    /// server.start("127.0.0.1:9090").await;
    /// ```
    async fn start(&mut self, socket_addr_str: String) {
        let addr = match socket_addr_str.parse::<SocketAddr>() {
            Ok(a) => a,
            Err(_) => {
                error!("[HTTP Server ID: {}] Can't parse socket_addr_str to std::net::SocketAddr!", self.server_id);
                return;
            }
        };
        self.addr = Some(addr);

        let (tx, mut rx) = mpsc::channel::<ServerCommand>(1);
        self.control_tx = Some(tx);

        loop {
            info!("[HTTP Server ID: {}] Starting server instance...", self.server_id);

            self.boot_time = Some(std::time::Instant::now());

            let handle = Handle::new();
            self.server_handle = Some(handle.clone());

            let app = Router::new()
                .fallback(any(handle_request))
                .with_state(self.interpreter.clone());

            let make_service = app.into_make_service();

            let server_handle_clone = handle.clone();
            let is_tls = self.is_tls_enabled();
            let rustls_config = self.rustls_config.clone();
            let server_id = self.server_id.clone();

            let server_task = tokio::spawn(async move {
                if is_tls {
                    let raw_cfg = match rustls_config {
                        Some(cfg) => cfg,
                        None => {
                            let err_msg = format!("[HTTP Server ID: {}] TLS is enabled, but tls_config is None!", server_id);
                            error!("{}", err_msg);
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Other, err_msg
                            ))
                        }
                    };
                    let config = axum_server::tls_rustls::RustlsConfig::from_config(raw_cfg);
                    axum_server::bind_rustls(addr, config)
                        .handle(server_handle_clone)
                        .serve(make_service)
                        .await
                } else {
                    axum_server::bind(addr)
                        .handle(server_handle_clone)
                        .serve(make_service)
                        .await
                }
            });

            let mut next_action = ServerCommand::Restart;

            let mut task_opt = Some(server_task);

            tokio::select! {
                maybe_command = rx.recv() => {
                    if let Some(command) = maybe_command {
                        next_action = command;
                        info!("[HTTP Server ID: {}] Control command received: {:?}", self.server_id, command);
                        handle.graceful_shutdown(Some(Duration::from_secs(5)));
                        
                        if let Some(task) = task_opt.take() {
                            let _ = task.await;
                        }
                    }
                }
                server_result = async {
                    if let Some(ref mut task) = task_opt {
                        task.await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    task_opt.take();
                    
                    match server_result {
                        Ok(Err(e)) => error!("[HTTP Server ID: {}] Server stopped with error: {}", self.server_id, e),
                        Err(panic_err) => error!("[HTTP Server ID: {}] Server task panicked: {:?}", self.server_id, panic_err),
                        _ => info!("[HTTP Server ID: {}] Server instance stopped gracefully.", self.server_id),
                    }
                    next_action = ServerCommand::Restart;
                }
            }

            match next_action {
                ServerCommand::Restart => {
                    info!("[HTTP Server ID: {}] Server restart triggered. Restarting...", self.server_id);
                    tokio::time::sleep(Duration::from_millis(250)).await
                }
                ServerCommand::Stop => {
                    info!("[HTTP Server ID: {}] Server stop triggered. Stopping...", self.server_id);
                    
                    self.server_handle = None;
                    self.control_tx = None;
                    self.boot_time = None;
                    break;
                }
            }
        }
    }

    /// Restart server if it running.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// let mut server = HttpServer::from_interpreter(interpreter, None, "1234".to_string());
    /// server.start("127.0.0.1:9090");
    /// 
    /// tokio::time::sleep(Duration::from_secs(5));
    /// 
    /// server.restart();
    /// ```
    async fn restart(&mut self) {
        info!("[HTTP Server ID: {}] Initializing restart...", self.server_id);

        if let Some(tx) = &self.control_tx {
            if let Err(e) = tx.send(ServerCommand::Restart).await {
                error!(
                    "[HTTP Server ID: {}] Failed to send restart signal to server loop {:?}",
                    self.server_id,
                    e
                )
            }
        } else {
            warn!(
                "[HTTP Server ID: {}] Server is not running or channel not initialized",
                self.server_id
            );
        }
    }

    /// Shutdown server if it running.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// let mut server = HttpServer::from_interpreter(interpreter, None, "1234".to_string());
    /// server.start("127.0.0.1:9090");
    /// 
    /// tokio::time::sleep(Duration::from_secs(5));
    /// 
    /// server.shutdown();
    /// ```
    async fn shutdown(&mut self) {
        info!("[HTTP Server ID: {}] Initializing shutdown...", self.server_id);

        if let Some(tx) = &self.control_tx {
            if let Err(e) = tx.send(ServerCommand::Stop).await {
                error!(
                    "[HTTP Server ID: {}] Failed to send stop signal to server loop {:?}",
                    self.server_id,
                    e
                )
            }
        } else {
            warn!(
                "[HTTP Server ID: {}] Server is not running or channel not initialized",
                self.server_id
            );
        }
    }

    /// Return statistics of server.
    /// 
    /// If server is not running, `uptime` parameter is zero (0)
    async fn stats(&self) -> super::ServerStats {
        super::ServerStats {
            id: self.server_id.clone(),
            uptime: {
                match self.boot_time {
                    Some(time) => time.elapsed().as_secs(),
                    None => 0,
                }
            }
        }
    }
}

#[axum::debug_handler]
async fn handle_request(
    State(interpreter): State<Option<Arc<Mutex<Interpreter>>>>,
    req: Request<Body>,
) -> impl IntoResponse {
    let (parts, body) = req.into_parts();

    let Some(interpreter) = interpreter else {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    {
        if interpreter.lock().is_err() {
            error!("[HTTP Server :: Handle Request] Failed to lock interpreter");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR, 
                "Internal Server Error!"
            ).into_response();
        }
    }

    let mut params = HashMap::new();
    if let Some(query_str) = parts.uri.query() {
        if let Ok(p) = serde_urlencoded::from_str::<HashMap<String, String>>(query_str) {
            params = p;
        }
    }

    let converted_headers = header_map_into_hashmap(&parts.headers);

    let rdl_body = make_rdl_body(body, &parts.headers).await
        .unwrap_or_else(|e| {
            error!("[HTTP Server :: Handle Request] Failed while parsing body: {}", e);
            HttpBodyVariant::Empty
        });

    let lock = match interpreter.lock() {
        Ok(l) => l,
        Err(_) => {
            error!("[HTTP Server :: Handle Request] Failed to lock interpreter");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR, 
                "Internal Server Error!"
            ).into_response();
        }
    };

    let response = lock.handle_request(
        &parts.method.to_string(), 
        parts.uri.path(), 
        params, 
        converted_headers, 
        rdl_body
    );

    (
        StatusCode::from_u16(response.status).unwrap_or(StatusCode::OK),
        response.body.unwrap_or("".to_string())
    ).into_response()
}


fn header_map_into_hashmap(map: &HeaderMap) -> HashMap<String, String> {
    let mut headers: HashMap<String, String> = HashMap::new();

    for (name, value) in map.iter() {
        let key = name.as_str().to_string();
        let val_str = String::from_utf8_lossy(value.as_bytes()).into_owned();

        headers.entry(key)
            .and_modify(|existing| {
                existing.push_str(", ");
                existing.push_str(&val_str);
            })
            .or_insert(val_str);
    }

    headers
}

async fn make_rdl_body(body: axum::body::Body, headers: &axum::http::HeaderMap) -> Result<HttpBodyVariant, String> {
    if let Some(content_length) = headers.get(CONTENT_LENGTH) {
        if content_length == "0" {
            return Ok(HttpBodyVariant::Empty);
        }
    }

    let collected = body.collect().await
        .map_err(|_| format!("[HTTP Server :: Packet Read] Failed while reading request"))?;

    let bytes = collected.to_bytes();

    if bytes.is_empty() {
        return Ok(HttpBodyVariant::Empty);
    }

    match String::from_utf8(bytes.to_vec()) {
        Ok(text) => Ok(HttpBodyVariant::Text(text)),
        Err(_) => Ok(HttpBodyVariant::Bytes(bytes.to_vec()))
    }
}