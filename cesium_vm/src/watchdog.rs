use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use crate::message::{Message, WatchdogReply};

pub trait WatchdogBackend {
    fn drop_downed_worker(&self, worker_id: usize);
    // fn start_new_worker_with_ctx(&self, ctx: )
}

pub enum DownAction {
    Restart,
    Shutdown,
}

pub struct Watchdog {
    pub target_tx: mpsc::Sender<Message>,
    pub timeout: Duration,
    pub action_on_down: DownAction,
    pub on_down_callback: Box<dyn FnOnce(usize)>,
}

impl Watchdog {
    pub fn new(
        target_tx: mpsc::Sender<Message>,
        timeout: Duration,
        action_on_down: DownAction,
        on_down_callback: impl FnOnce(usize) + 'static,
    ) -> Self {
        Self {
            target_tx, timeout, action_on_down,
            on_down_callback: Box::new(on_down_callback)
        }
    }

    pub async fn watch(&self) {
    //     loop {
    //         let (_, rx) = oneshot::channel::<WatchdogReply>();
    //
    //         self.target_tx.send(Message::Ping).await;
    //
    //         if let Err(_) = timeout(self.timeout, rx).await {
    //             match self.action_on_down {
    //                 DownAction::Restart => {
    //
    //                 }
    //             }
    //         }
    //
    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //     }
    }
}