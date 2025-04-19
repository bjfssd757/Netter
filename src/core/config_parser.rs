use serde::Deserialize;
use crate::core::config::{
    EnviromentConfigure,
    CacheConfigure,
    SslConfigure,
    PoolConfigure,
    Logger,
    MonitoringConfigure,
};
use std::fs;
use toml;

use super::config::RouteConfig;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub protect: bool,
    pub enviroment: EnviromentConfigure,
    pub cache: CacheConfigure,
    pub ssl: SslConfigure,
    pub pool: PoolConfigure,
    pub logger: Logger,
    pub monitoring: MonitoringConfigure,
    pub routes: Vec<RouteConfig>,
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::de::from_str(&content)?;
    Ok(config)
}