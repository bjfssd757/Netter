use std::{collections::HashMap, sync::{Arc, atomic::{AtomicU64, Ordering}}};

use log::{error, info, warn};
use tokio::{sync::{Mutex, mpsc}, task::JoinHandle};

use crate::language::compiler::vm::context::ProcessContext;

pub mod context;

pub struct VirtualMachine {
    processes: Arc<Mutex<HashMap<PID, ProcessInfo>>>,
    next_pid: AtomicU64,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: AtomicU64::new(0),
        }
    }

    pub async fn spawn_process(
        &self,
        context: ProcessContext,
        bytecode: Vec<u8>,
        ops_limit: u32,
    ) -> PID {
        let pid = PID(self.next_pid.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = mpsc::channel::<ProcessSignal>(10);

        let process_handle = context.start(bytecode, ops_limit);

        let processes_registry = self.processes.clone();

        let mut lock = self.processes.lock().await;

        let watchdog_handle = tokio::spawn(async move {
            VirtualMachine::run_watchdog(pid, process_handle, rx, processes_registry).await;
        });

        lock.insert(pid, ProcessInfo {
            watchdog_handle,
            signal_tx: tx,
        });

        pid
    }

    pub async fn run_watchdog(
        pid: PID,
        mut process_handle: JoinHandle<Result<ProcessContext, String>>,
        mut signal_rx: mpsc::Receiver<ProcessSignal>,
        processes_registry: Arc<Mutex<HashMap<PID, ProcessInfo>>>,
    ) {
        loop {
            tokio::select! {
                res = &mut process_handle => {
                    match res {
                        Ok(Ok(_ctx)) => {
                            info!("[Watchdog] Process {:?} successfully shut downed", pid);
                        }
                        Ok(Err(err)) => {
                            error!("[Watchdog] Process {:?} shut downed with error: {}", pid, err);

                            // TODO: restart process
                        }
                        Err(join_err) => {
                            if join_err.is_panic() {
                                error!("[Watchdog] CRITICAL PANIC in process {:?}! Virtual Machine code is downed", pid);
                            } else {
                                warn!("[Watchdog] Process {:?} shut downed manually", pid);
                            }
                        }
                    }
                    break;
                }

                Some(signal) = signal_rx.recv() => {
                    match signal {
                        ProcessSignal::Kill => {
                            info!("[Watchdog] Got Kill signal for process {:?}. Shut downing...", pid);
                            process_handle.abort();
                            break;
                        }
                    }
                }
            }
        }

        let mut lock = processes_registry.lock().await;
        lock.remove(&pid);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PID(pub u64);

pub enum ProcessSignal {
    Kill,
}

pub struct ProcessInfo {
    pub watchdog_handle: JoinHandle<()>, 
    pub signal_tx: mpsc::Sender<ProcessSignal>,
}