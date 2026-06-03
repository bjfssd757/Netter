use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::message::Message;

pub struct VMContext<const MEM_SIZE: usize> {
    memory: [u8; MEM_SIZE],
}

pub struct VM<const MEM_SIZE: usize> {
    channels: HashMap<usize, mpsc::Sender<Message>>,
    watchdog_ids: Vec<usize>,
    context: Arc<VMContext<MEM_SIZE>>
}