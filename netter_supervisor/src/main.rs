use netter_proto::IntoSendSync;
use netter_proto::supervisor::{SupervisorClient, SupervisorServer};

#[cfg(windows)]
const SOCKET_PATH_SUPERVISOR: &str = r"\\.\pipe\netter_supervisor";
#[cfg(windows)]
const SOCKET_PATH_VM: &str = r"\\.\pipe\netter_vm";

#[cfg(unix)]
const SOCKET_PATH_SUPERVISOR: &str = "/tmp/netter_supervisor.sock";
#[cfg(unix)]
const SOCKET_PATH_VM: &str = "/tmp/netter_virtual_machine.sock";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start().await
}

async fn start() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = SupervisorClient::connect_with_socket(SOCKET_PATH_VM).await?;
    let server = SupervisorServer::new(client);
    server.start_with_socket(SOCKET_PATH_SUPERVISOR).await.map_err(|e| e.into_send_sync())?;

    Ok(())
}

