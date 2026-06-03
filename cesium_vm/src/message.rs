pub enum Message {
    Shutdown,
    Restart,
    Transfer(Option<Vec<u8>>),
    Ping,
}

pub enum WatchdogReply {
    Pong {
        worker_id: usize,
    }
}