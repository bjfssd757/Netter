#![allow(async_fn_in_trait)]

use std::{fs::File, io::BufReader};
use log::{debug, error};
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde::{Deserialize, Serialize};
use crate::CoreError;

pub mod webcosket_core;
pub mod http_core;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: String,
    pub key_path: String,
}

#[allow(dead_code)]
pub struct ServerStats {
    id: String,
    uptime: u64,
}

pub trait Server {
    async fn start(&mut self, socket_addr_str: String);
    async fn restart(&mut self);
    async fn shutdown(&mut self);
    async fn stats(&self) -> ServerStats;
}

pub(crate) fn load_rustls_config(cert_path: &str, key_path: &str) -> Result<ServerConfig, CoreError> {
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