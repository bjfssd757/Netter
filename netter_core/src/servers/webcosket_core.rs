use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};

#[allow(async_fn_in_trait)]
pub trait WebSocketTrait {
    fn new(host: String, port: u16, protect: bool) -> Self;
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    host: String,
    port: u16,
    protect: bool,
}

impl WebSocketTrait for Server {
    fn new(host: String, port: u16, protect: bool) -> Self {
        Self {
            host,
            port,
            protect,
        }
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.protect {
            println!("Server is protected");
            Ok(()) // there should be code for ssl or tls protect
        }
        else {
            println!("Starting server...");

            let addr = format!("{}:{}", self.host, self.port);
            let listener = TcpListener::bind(&addr)
                .await
                .map_err(|e| format!("Failed to bind: {e}"))?;

            // save_state(
            //     String::from("websocket"),
            //     self.host.clone(),
            //     self.port.clone()
            // )?;

            println!("Server running on {}", &addr);

            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let ws_stream =  match accept_async(stream)
                        .await {
                            Ok(ws) => ws,
                            Err(e) => {
                                eprintln!("Error during WebSocket handshake: {}", e);
                                return
                            }
                        };

                        println!("New connection!");

                    let (mut write, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(msg) => {
                                println!("Received message: {}", msg);
                                if msg.is_text() || msg.is_binary() {
                                    write.send(msg).await.unwrap();
                                }
                            },
                            Err(e) => {
                                eprintln!("Failed while reading message: {e}");
                                return
                            }
                        }
                    }
                });
            }
            Ok(())
        }
    }
}