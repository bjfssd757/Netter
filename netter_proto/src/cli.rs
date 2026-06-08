use prost_types::Duration;
use tonic::codegen::StdError;
use tonic::Request;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use crate::{
    proto_cli::v1::{
        cli_service_client::CliServiceClient,
        GetInfoAboutServerRequest,
    },
    proto_shared::v1::{
        RestartServerRequest,
        Server,
        StartServerRequest,
        StopServerRequest,
    },
};

/// Console gRPC client for interacting with the Supervisor.
pub struct CliClient {
    inner: CliServiceClient<Channel>,
}

impl CliClient {
    pub async fn connect_with_socket(path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = Endpoint::from_static("http://[::]:50051");

        let path_str = path.to_string();

        let channel = endpoint.connect_with_connector(service_fn(move |_: Uri| {
            let path = path_str.clone();
            async move {
                #[cfg(unix)]
                {
                    let stream = tokio::net::UnixStream::connect(path).await?;
                    Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(stream))
                }

                #[cfg(windows)]
                {
                    use tokio::net::windows::named_pipe::ClientOptions;
                    use std::time::Duration;

                    const ERROR_PIPE_BUSY: i32 = 231;

                    let client = loop {
                        match ClientOptions::new().open(&path) {
                            Ok(client) => break client,
                            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                                tokio::time::sleep(Duration::from_millis(20)).await;
                            }
                            Err(e) => return Err(e)
                        }
                    };

                    Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(client))
                }
            }
        })).await?;

        Ok(Self {
            inner: CliServiceClient::new(channel)
        })
    }

    pub async fn connect<D>(dst: D) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<StdError>,
    {
        let endpoint = dst.try_into().map_err(|e| e.into())?;
        let channel = endpoint.connect().await?;
        Ok(Self {
            inner: CliServiceClient::new(channel),
        })
    }

    pub async fn get_server_info(&self, server_id: u32) -> Result<Server, String> {
        let request = Request::new(GetInfoAboutServerRequest {
            server_id,
        });

        let mut client = self.inner.clone();

        match client.get_info_about_server(request).await {
            Ok(response) => {
                let res = response.into_inner();
                Ok(res.server_info
                    .ok_or_else(|| "Server return empty data for request".to_string())?)
            }
            Err(status) => Err(format!(
                "Error gRPC [{}]: {}",
                status.code(),
                status.message(),
            )),
        }
    }

    pub async fn ping_supervisor(&self) -> Result<(), String> {
        let mut client = self.inner.clone();

        match client.ping_supervisor(()).await {
            Ok(_) => Ok(()),
            Err(status) => Err(format!(
                "Error gRPC: [{}]: {}",
                status.code(),
                status.message(),
            ))
        }
    }

    pub async fn ping_virtual_machine(&self) -> Result<(), String> {
        let mut client = self.inner.clone();

        match client.ping_virtual_machine(()).await {
            Ok(_) => Ok(()),
            Err(status) => Err(format!(
                "Error gRPC: [{}]: {}",
                status.code(),
                status.message(),
            ))
        }
    }

    pub async fn start_server(&self, server: Server) -> Result<u32, String> {
        let request = Request::new(StartServerRequest {
            server: Some(server),
        });

        let mut client = self.inner.clone();

        match client.start_server(request).await {
            Ok(response) => {
                let res = response.into_inner();
                Ok(res.server_id)
            }
            Err(status) => Err(format!(
                "gRPC Error [{}]: {}",
                status.code(),
                status.message(),
            ))
        }
    }

    pub async fn stop_server(&self, server_id: u32) -> Result<(), String> {
        let request = Request::new(StopServerRequest {
            server_id,
        });

        let mut client = self.inner.clone();

        match client.stop_server(request).await {
            Ok(_) => Ok(()),
            Err(status) => Err(format!(
                "gRPC Error [{}]: {}",
                status.code(),
                status.message(),
            ))
        }
    }

    pub async fn restart_server(&self, server_id: u32, wait_before_start: Option<Duration>) -> Result<u32, String> {
        let request = Request::new(RestartServerRequest {
            server_id,
            wait_before_start,
        });

        let mut client = self.inner.clone();

        match client.restart_server(request).await {
            Ok(response) => {
                let res = response.into_inner();
                Ok(res.new_server_id)
            }
            Err(status) => Err(format!(
                "gRPC Error [{}]: {}",
                status.code(),
                status.message(),
            ))
        }
    }
}