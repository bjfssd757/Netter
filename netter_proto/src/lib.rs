#[cfg(feature = "cli_client")]
pub mod cli;

#[cfg(feature = "supervisor")]
pub mod supervisor;

#[cfg(feature = "vm_server")]
pub mod vm;

pub mod proto_shared {
    pub mod v1 {
        tonic::include_proto!("netter.shared.v1");
    }
}

pub mod proto_supervisor {
    pub mod v1 {
        tonic::include_proto!("netter.supervisor.v1");
    }
}

pub mod proto_cli {
    pub mod v1 {
        tonic::include_proto!("netter.cli.v1");
    }
}

use std::net::SocketAddr;
use prost_types::Duration;
use std::time::SystemTime;
use proto_shared::v1::{
    Server,
    LogConfiguration,
    log_configuration::Backend,
    Route,
    TlsConfiguration
};

pub struct ServerBuilder {
    inner: Server,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            inner: Server::default(),
        }
    }

    pub fn build(self) -> Server {
        self.inner
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.inner.ip = ip;
        self
    }

    pub fn with_port(mut self, port: u32) -> Self {
        self.inner.port = port;
        self
    }

    pub fn with_ip_and_port(mut self, ip: String, port: u32) -> Self {
        self.inner.ip = ip;
        self.inner.port = port;
        self
    }

    pub fn with_address(mut self, addr: String) -> Result<Self, Box<dyn std::error::Error>> {
        let addr: SocketAddr = addr.parse()?;

        self.inner.ip = addr.ip().to_string();
        self.inner.port = addr.port() as u32;
        Ok(self)
    }

    pub fn with_routes(mut self, routes: Vec<Route>) -> Self {
        self.inner.routes = routes;
        self
    }

    pub fn add_route(mut self, route: Route) -> Self {
        self.inner.routes.push(route);
        self
    }

    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.inner.connection_timeout = Some(timeout);
        self
    }

    pub fn with_tls_config(mut self, tls_cfg: TlsConfiguration) -> Self {
        self.inner.tls_config = Some(tls_cfg);
        self
    }

    pub fn with_log_config(mut self, log_cfg: LogConfiguration) -> Self {
        self.inner.log_config = Some(log_cfg);
        self
    }
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }
}

pub struct LogConfigurationBuilder {
    inner: LogConfiguration
}

impl LogConfigurationBuilder {
    pub fn new() -> Self {
        Self {
            inner: LogConfiguration::default(),
        }
    }

    pub fn with_backend(mut self, backend: Backend) -> Self {
        self.inner.backend = Some(backend);
        self
    }

    pub fn build(self) -> LogConfiguration {
        self.inner
    }
}

impl LogConfiguration {
    pub fn builder() -> LogConfigurationBuilder {
        LogConfigurationBuilder::new()
    }

    pub fn new(backend: Backend) -> Self {
        Self {
            backend: Some(backend),
        }
    }
}

pub struct TlsConfigurationBuilder {
    inner: TlsConfiguration,
}

impl TlsConfigurationBuilder {
    pub fn new() -> Self {
        Self {
            inner: TlsConfiguration::default(),
        }
    }

    pub fn with_key(mut self, key: String) -> Self {
        self.inner.key = key;
        self
    }

    pub fn with_pem(mut self, pem: String) -> Self {
        self.inner.pem = pem;
        self
    }

    pub fn build(self) -> TlsConfiguration {
        self.inner
    }
}

impl TlsConfiguration {
    pub fn builder() -> TlsConfigurationBuilder {
        TlsConfigurationBuilder::new()
    }

    pub fn new(key: String, pem: String) -> Self {
        Self {
            key, pem,
        }
    }
}

pub trait ServerExt {
    fn uptime(&self) -> Option<std::time::Duration>;
}

impl ServerExt for Server {
    fn uptime(&self) -> Option<std::time::Duration> {
        let started_at = self.started_at.as_ref()?;
        let start_system_time: SystemTime = (*started_at).try_into().ok()?;

        start_system_time.elapsed().ok()
    }
}