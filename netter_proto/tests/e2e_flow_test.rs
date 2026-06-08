use tonic::Status;
use std::sync::Arc;
use std::time::Duration;
use netter_proto::async_cb;
use netter_proto::cli::CliClient;
use netter_proto::proto_shared::v1::{Server, StartServerRequest, StartServerResponse};
use netter_proto::supervisor::{SupervisorClient, SupervisorServer};
use netter_proto::vm::VirtualMachineServer;
use netter_proto_macros::async_callback;

struct Context;

#[async_callback]
async fn success_start_cb(
    _ctx: Arc<Context>,
    _req: StartServerRequest
) -> Result<StartServerResponse, Status> {
    tokio::time::sleep(Duration::from_secs(3)).await;
    Ok(StartServerResponse { server_id: 2 })
}

#[tokio::test]
async fn e2e_success_supervisor_to_vm_flow() {
    tokio::spawn(async move {
        VirtualMachineServer::new(Context)
            .with_start_server(success_start_cb)
            .with_ping(|_| {})
            .with_get_runtime_info(async_cb!(|ctx, req| {
                Err(Status::unimplemented(""))
            }))
            .with_stop_server(async_cb!(|ctx, req| {
                Err(Status::unimplemented(""))
            }))
            .with_restart_server(async_cb!(|ctx, req| {
                Err(Status::unimplemented(""))
            }))
            .build()
            .start("127.0.0.1:50051")
            .await.expect("Failed to start server");
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = SupervisorClient::connect("http://127.0.0.1:50051").await
        .expect("Failed to connect to the server");

    let server = Server::default();

    let id = client.start_server(server).await.expect("Failed to send request `start_server`");

    assert_eq!(id, 2);
}

#[tokio::test]
async fn e2e_success_cli_to_vm_flow() {
    tokio::spawn(async move {
        VirtualMachineServer::new(Context)
            .with_start_server(success_start_cb)
            .with_ping(|_| {})
            .with_get_runtime_info(async_cb!(|ctx, req| {
                Err(Status::unimplemented(""))
            }))
            .with_stop_server(async_cb!(|ctx, req| {
                Err(Status::unimplemented(""))
            }))
            .with_restart_server(async_cb!(|ctx, req| {
                Err(Status::unimplemented(""))
            }))
            .build()
            .start("127.0.0.1:50052")
            .await.expect("Failed to start server");
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    tokio::spawn(async move {
        let client = SupervisorClient::connect("http://127.0.0.1:50052")
            .await.expect("Failed to connect SupervisorClient to VMServer");

        SupervisorServer::new(client).start_with_address("127.0.0.1:50053").await
            .expect("Failed to start SupervisorServer");
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = CliClient::connect("http://127.0.0.1:50053").await
        .expect("Failed to connect to the server");

    let server = Server::default();

    let id = client.start_server(server).await.expect("Failed to send request `start_server`");

    assert_eq!(id, 2);
}

#[tokio::test]
async fn e2e_success_cli_to_vm_socket_flow() {
    tokio::spawn(async move {
        VirtualMachineServer::new(Context)
            .with_start_server(success_start_cb)
            .with_ping(|_| {})
            .with_get_runtime_info(async_cb!(|ctx, req| {
                Err(Status::unimplemented(""))
            }))
            .with_stop_server(async_cb!(|ctx, req| {
                Err(Status::unimplemented(""))
            }))
            .with_restart_server(async_cb!(|ctx, req| {
                Err(Status::unimplemented(""))
            }))
            .build()
            .start_with_socket(r"\\.\pipe\netter_vm")
            .await.expect("Failed to start server");
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    tokio::spawn(async move {
        let client = SupervisorClient::connect_with_socket(r"\\.\pipe\netter_vm")
            .await.expect("Failed to connect SupervisorClient to VMServer");

        SupervisorServer::new(client).start_with_socket(r"\\.\pipe\netter_supervisor").await
            .expect("Failed to start SupervisorServer");
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = CliClient::connect_with_socket(r"\\.\pipe\netter_supervisor").await
        .expect("Failed to connect to the server");

    let server = Server::default();

    let id = client.start_server(server).await.expect("Failed to send request `start_server`");

    assert_eq!(id, 2);
}