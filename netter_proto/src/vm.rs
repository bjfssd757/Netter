use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use crate::proto_shared::v1::{GetRuntimeInfoRequest, GetRuntimeInfoResponse, RestartServerRequest, RestartServerResponse, StartServerRequest, StartServerResponse, StopServerRequest, StopServerResponse};
use crate::proto_supervisor::v1::supervisor_service_server::{SupervisorService, SupervisorServiceServer};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'a>>;
pub type Callback<CTX, Req, Res> = fn(Arc<CTX>, Req) -> BoxFuture<'static, Result<Res, Status>>;

pub struct VirtualMachineServer<CTX> {
    ctx: Option<Arc<CTX>>,
    is_built: bool,
    ping_callback: Option<fn(Arc<CTX>)>,
    get_runtime_info_callback: Option<Callback<CTX, GetRuntimeInfoRequest, GetRuntimeInfoResponse>>,
    start_server_callback: Option<Callback<CTX, StartServerRequest, StartServerResponse>>,
    stop_server_callback: Option<Callback<CTX, StopServerRequest, StopServerResponse>>,
    restart_server_callback: Option<Callback<CTX, RestartServerRequest, RestartServerResponse>>,
}

impl<CTX: Send + Sync + 'static> VirtualMachineServer<CTX> {
    /// Start Virtual Machine Server work on given address.
    ///
    /// # Panics
    /// This function may panic if [Self::build()] is not called
    pub async fn start(self, address: impl Into<String>) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_built {
            panic!("[VM] VirtualMachineServer is not built!");
        }

        let addr: SocketAddr = address.into().parse()?;

        tonic::transport::Server::builder()
            .add_service(SupervisorServiceServer::new(self))
            .serve(addr)
            .await?;

        Ok(())
    }
}

impl<CTX> VirtualMachineServer<CTX> {
    pub fn default(ctx: CTX) -> Self {
        Self {
            ctx: Some(Arc::new(ctx)),
            is_built: false,
            restart_server_callback: None,
            stop_server_callback: None,
            ping_callback: None,
            start_server_callback: None,
            get_runtime_info_callback: None,
        }
    }

    pub fn new(ctx: CTX) -> Self {
        Self::default(ctx)
    }
}

impl<CTX> VirtualMachineServer<CTX> {
    pub const fn with_ping(mut self, cb: fn(Arc<CTX>)) -> Self {
        self.ping_callback = Some(cb);
        self
    }

    pub const fn with_start_server(
        mut self,
        cb: Callback<CTX, StartServerRequest, StartServerResponse>
    ) -> Self {
        self.start_server_callback = Some(cb);
        self
    }

    pub const fn with_get_runtime_info(
        mut self,
        cb: Callback<CTX, GetRuntimeInfoRequest, GetRuntimeInfoResponse>
    ) -> Self {
        self.get_runtime_info_callback = Some(cb);
        self
    }

    pub const fn with_stop_server(
        mut self,
        cb: Callback<CTX, StopServerRequest, StopServerResponse>
    ) -> Self {
        self.stop_server_callback = Some(cb);
        self
    }

    pub const fn with_restart_server(
        mut self,
        cb: Callback<CTX, RestartServerRequest, RestartServerResponse>
    ) -> Self {
        self.restart_server_callback = Some(cb);
        self
    }

    pub const fn build(mut self) -> Self {
        if self.stop_server_callback.is_none() {
            panic!("[VM] `stop_server_callback` is not set. Please, init this callback from `with_stop_server` function");
        }
        if self.ping_callback.is_none() {
            panic!("[VM] `ping_callback` is not set. Please, init this callback from `with_ping` function");
        }
        if self.get_runtime_info_callback.is_none() {
            panic!("[VM] `get_runtime_info_callback` is not set. Please, init this callback from `with_get_runtime_info` function");
        }
        if self.start_server_callback.is_none() {
            panic!("[VM] `start_server_callback` is not set. Please, init this callback from `with_start_server` function");
        }
        if self.restart_server_callback.is_none() {
            panic!("[VM] `restart_server_callback` is not set. Please, init this callback from `with_restart_server` function");
        }

        self.is_built = true;
        self
    }
}


#[tonic::async_trait]
impl<CTX: Send + Sync + 'static> SupervisorService for VirtualMachineServer<CTX> {
    async fn get_runtime_info(&self, request: Request<GetRuntimeInfoRequest>) -> Result<Response<GetRuntimeInfoResponse>, Status> {
        if let (Some(cb), Some(ctx)) = (self.get_runtime_info_callback, &self.ctx) {

            let future = cb(Arc::clone(ctx), request.into_inner());

            return match future.await {
                Ok(resp) => Ok(Response::new(resp)),
                Err(status) => Err(status)
            }
        }
        Err(Status::not_found("Function is not implemented"))
    }

    async fn start_server(&self, request: Request<StartServerRequest>) -> Result<Response<StartServerResponse>, Status> {
        if let (Some(cb), Some(ctx)) = (self.start_server_callback, &self.ctx) {

            let future = cb(Arc::clone(ctx), request.into_inner());

            return match future.await {
                Ok(resp) => Ok(Response::new(resp)),
                Err(status) => Err(status),
            }
        }
        Err(Status::not_found("Function is not implemented"))
    }

    async fn stop_server(&self, request: Request<StopServerRequest>) -> Result<Response<StopServerResponse>, Status> {
        if let (Some(cb), Some(ctx)) = (self.stop_server_callback, &self.ctx) {

            let future = cb(Arc::clone(ctx), request.into_inner());

            return match future.await {
                Ok(resp) => Ok(Response::new(resp)),
                Err(status) => Err(status)
            }
        }
        Err(Status::not_found("Function is not implemented"))
    }

    async fn restart_server(&self, request: Request<RestartServerRequest>) -> Result<Response<RestartServerResponse>, Status> {
        if let (Some(cb), Some(ctx)) = (self.restart_server_callback, &self.ctx) {

            let future = cb(Arc::clone(ctx), request.into_inner());

            return match future.await {
                Ok(resp) => Ok(Response::new(resp)),
                Err(status) => Err(status)
            }
        }
        Err(Status::not_found("Function is not implemented"))
    }

    async fn ping(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        if let (Some(cb), Some(ctx)) = (self.ping_callback, &self.ctx) {
            cb(Arc::clone(ctx));
            return Ok(Response::new(()))
        }
        Err(Status::not_found("Function is not implemented"))
    }
}


#[macro_export]
macro_rules! async_cb {
    (|$ctx:ident, $req:ident| $body:block) => {
        |$ctx, $req| {
            Box::pin(async move { $body })
        }
    };
}