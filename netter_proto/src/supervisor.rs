use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use prost_types::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_stream::Stream;
use tonic::codegen::StdError;
use tonic::{Request, Response, Status};
use tonic::transport::{Channel, Endpoint, Uri};
use tonic::transport::server::Connected;
use tower::service_fn;
use crate::proto_cli::v1::cli_service_server::{CliService, CliServiceServer};
use crate::proto_cli::v1::{GetInfoAboutServerRequest, GetInfoAboutServerResponse};
use crate::proto_shared::v1::{GetRuntimeInfoRequest, RestartServerRequest, RestartServerResponse, Server, StartServerRequest, StartServerResponse, StopServerRequest, StopServerResponse};
use crate::proto_supervisor::v1::supervisor_service_client::SupervisorServiceClient;

pub enum CrossPlatformStream {
    #[cfg(unix)]
    Uds(tokio::net::UnixStream),
    #[cfg(windows)]
    NamedPipe(tokio::net::windows::named_pipe::NamedPipeServer),
}

impl AsyncRead for CrossPlatformStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match &mut *self {
            #[cfg(unix)]
            Self::Uds(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(windows)]
            Self::NamedPipe(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for CrossPlatformStream {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match &mut *self {
            #[cfg(unix)]
            Self::Uds(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(windows)]
            Self::NamedPipe(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match &mut *self {
            #[cfg(unix)]
            Self::Uds(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(windows)]
            Self::NamedPipe(stream) => Pin::new(stream).poll_flush(cx),
        }
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match &mut *self {
            #[cfg(unix)]
            Self::Uds(stream) => Pin::new(stream).poll_shutdown(cx),
            #[cfg(windows)]
            Self::NamedPipe(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

impl Connected for CrossPlatformStream {
    type ConnectInfo = ();
    fn connect_info(&self) -> Self::ConnectInfo {
        ()
    }
}

/// Supervisor proxy server (gRPC API Gateway) serving CLI clients.
pub struct SupervisorServer {
    client: SupervisorClient
}

impl SupervisorServer {
    pub fn new(client: SupervisorClient) -> Self {
        Self {
            client,
        }
    }

    /// Start SupervisorServer on given pipe or uds (Unix Domain Sockets)
    pub async fn start_with_socket(
        self,
        path: impl Into<String>
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.into();

        let svc = CliServiceServer::new(self);
        let router = tonic::transport::Server::builder().add_service(svc);

        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(&path);
            let listener = tokio::net::UnixListener::bind(&path)?;

            let incoming: Pin<Box<dyn Stream<Item = Result<CrossPlatformStream, Box<dyn std::error::Error + Send + Sync>>> + Send>> =
                Box::pin(async_stream::try_stream! {
                loop {
                    let (stream, _) = listener.accept().await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    yield CrossPlatformStream::Uds(stream);
                }
            });
            router.serve_with_incoming(incoming).await?;
        }

        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ServerOptions;


            let pipe_name = path.clone();
            let incoming: Pin<Box<dyn Stream<Item = Result<CrossPlatformStream, Box<dyn std::error::Error + Send + Sync>>> + Send>> =
                Box::pin(async_stream::try_stream! {
                let mut is_first = true;
                loop {
                    let server = ServerOptions::new()
                        .first_pipe_instance(is_first)
                        .create(&pipe_name)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                    is_first = false;

                    server.connect().await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    yield CrossPlatformStream::NamedPipe(server);
                }
            });
            router.serve_with_incoming(incoming).await?;
        }

        Ok(())
    }

    /// Start SupervisorServer on given address (for example: 127.0.0.1:50051)
    pub async fn start_with_address(
        self,
        address: impl Into<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let addr: SocketAddr = address.into().parse()?;

        tonic::transport::Server::builder()
            .add_service(CliServiceServer::new(self))
            .serve(addr)
            .await?;

        Ok(())
    }
}

#[tonic::async_trait]
impl CliService for SupervisorServer {
    async fn ping_supervisor(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn ping_virtual_machine(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        match self.client.ping().await {
            Ok(()) => Ok(Response::new(())),
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn get_info_about_server(&self, request: Request<GetInfoAboutServerRequest>) -> Result<Response<GetInfoAboutServerResponse>, Status> {
        let req = request.into_inner();
        let id = req.server_id;

        match self.client.get_runtime_info(id).await {
            Ok(server) => Ok(Response::new(GetInfoAboutServerResponse { server_info: server })),
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn start_server(&self, request: Request<StartServerRequest>) -> Result<Response<StartServerResponse>, Status> {
        let req = request.into_inner();
        let server = req.server;

        if let Some(s) = server {
            return match self.client.start_server(s).await {
                Ok(id) => Ok(Response::new(StartServerResponse { server_id: id })),
                Err(e) => Err(Status::internal(e)),
            }
        }
        Err(Status::invalid_argument("`server` field in argument is None"))
    }

    async fn stop_server(&self, request: Request<StopServerRequest>) -> Result<Response<StopServerResponse>, Status> {
        let req = request.into_inner();
        let id = req.server_id;

        match self.client.stop_server(id).await {
            Ok(_) => Ok(Response::new(StopServerResponse {})),
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn restart_server(&self, request: Request<RestartServerRequest>) -> Result<Response<RestartServerResponse>, Status> {
        let req = request.into_inner();
        let id = req.server_id;
        let wait = if let Some(duration) = req.wait_before_start {
            duration
        } else {
            Duration::default()
        };

        match self.client.restart_server(id, wait).await {
            Ok(id) => Ok(Response::new(RestartServerResponse { new_server_id: id })),
            Err(e) => Err(Status::internal(e)),
        }
    }
}

/// High-level gRPC client of Supervisor for sending commands to the Virtual Machine.
pub struct SupervisorClient {
    inner: SupervisorServiceClient<Channel>,
}
impl SupervisorClient {
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
            inner: SupervisorServiceClient::new(channel)
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
            inner: SupervisorServiceClient::new(channel),
        })
    }

    pub async fn ping(&self) -> Result<(), String> {
        let mut client = self.inner.clone();

        match client.ping(()).await {
            Ok(_) => Ok(()),
            Err(status) => Err(format!(
                "Error gRPC [{}]: {}",
                status.code(),
                status.message(),
            )),
        }
    }

    pub async fn get_runtime_info(&self, server_id: u32) -> Result<Option<Server>, String> {
        let request = Request::new(GetRuntimeInfoRequest {
            server_id,
        });

        let mut client = self.inner.clone();

        match client.get_runtime_info(request).await {
            Ok(response) => {
                let res = response.into_inner();
                Ok(res.server)
            }
            Err(status) => Err(format!(
                "Error gRPC [{}]: {}",
                status.code(),
                status.message(),
            )),
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
                "Error gRPC [{}]: {}",
                status.code(),
                status.message(),
            )),
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
                "Error gRPC [{}]: {}",
                status.code(),
                status.message(),
            )),
        }
    }

    pub async fn restart_server(&self, server_id: u32, wait_before_start: Duration) -> Result<u32, String> {
        let request = Request::new(RestartServerRequest {
            server_id,
            wait_before_start: Some(wait_before_start),
        });

        let mut client = self.inner.clone();

        match client.restart_server(request).await {
            Ok(response) => {
                let res = response.into_inner();
                Ok(res.new_server_id)
            }
            Err(status) => Err(format!(
                "Error gRPC [{}]: {}",
                status.code(),
                status.message(),
            )),
        }
    }
}
