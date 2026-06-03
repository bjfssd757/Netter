use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::context_worker::WorkerContext;
use crate::message::Message;

pub struct Worker<
    OP,
    EXEC,
    const MAX_PROPERTIES: usize = 256,
    const MAX_FUNCTIONS: usize = 64,
    const MAX_ARGS_FUNCTIONS: usize = 8,
>
where
    OP: Opcode,
    EXEC: WorkerExecute,
{
    rx: mpsc::Receiver<Message>,
    code: Arc<[u8]>,
    context: WorkerContext<MAX_PROPERTIES, MAX_FUNCTIONS, MAX_ARGS_FUNCTIONS>,

    op_phantom: PhantomData<OP>,
    exec_phantom: PhantomData<EXEC>,
}

pub trait WorkerExecute: Sized {
    type OP: Opcode;
    fn execute(&self, worker: &mut Worker<Self::OP, Self>) -> Self::OP;
}

pub trait Opcode: Sized {
    fn as_bytes(&self) -> impl Into<Arc<[u8]>>;
}

impl<OP, EXEC> Worker<OP, EXEC>
where
    OP: Opcode,
    EXEC: WorkerExecute,
{
    // pub fn new(
    //     rx: mpsc::Receiver<Message>,
    //     payload: impl Into<Arc<[u8]>>,
    // ) -> Self {
    //     Self {
    //         rx,
    //         code: payload.into(),
    //         exec_phantom: PhantomData,
    //         op_phantom: PhantomData,
    //     }
    // }

    /// Start execution of worker. You can call this function in async execution context. \
    /// # Example
    /// ```rust
    /// tokio::spawn(async move {
    ///     worker.start().await?;
    /// });
    /// ```
    pub async fn start(&self) -> Result<(), std::io::Error> {
        Ok(())
    }
}