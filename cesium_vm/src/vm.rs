use std::sync::Arc;
use wasmtime::{Config, Engine, Instance, InstanceAllocationStrategy, Module, PoolingAllocationConfig, Store};

pub enum VMError {
    WASMEngineCreateError,
    WASMEntryPointNotFound,
    WASMFailedToCallFunction,
    WASMFailedToParseFunctionResponse,
    WASMInvalidStatusReturned,
    WASMFailedToGetMemory,
    WASMFailedWhileWritingInMemory,
    WASMFailedToGetInstance,
    WASMProvidedWebAssemblyBytecodeIsNotValid,
}

pub enum WorkerResult {
    Success {
        logs: String,
        output: Vec<u8>,
        is_reload_required: bool,
    },
    Failed {
        is_memory_corrupted: bool,
        is_critical_error: bool,
    },
}

pub struct VM {
    engine: Arc<Engine>,
}

impl VM {
    pub fn new(
        max_workers: u32,
        max_memory_size_bytes: usize
    ) -> Result<Self, VMError> {
        let mut pooling_config = PoolingAllocationConfig::default();
        pooling_config.total_memories(max_workers);
        pooling_config.max_memory_size(max_memory_size_bytes);

        let mut config = Config::new();
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pooling_config));


        let engine = Engine::new(&config).map_err(|_| VMError::WASMEngineCreateError)?;

        Ok(Self {
            engine: Arc::new(engine),
        })
    }

    /// Execute worker with given wasm bytes and context in bytes
    pub fn run_worker(&self, wasm_bytes: &[u8], context_bytes: &[u8]) -> Result<WorkerResult, VMError> {
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|_| VMError::WASMProvidedWebAssemblyBytecodeIsNotValid)?;

        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|_| VMError::WASMFailedToGetInstance)?;

        let memory = instance.get_memory(&mut store, "memory")
            .ok_or_else(|| VMError::WASMFailedToGetMemory)?;

        memory.write(&mut store, 0, context_bytes)
            .map_err(|_| VMError::WASMFailedWhileWritingInMemory)?;

        let entry_point = instance.get_typed_func::<(u32, u32), u64>(
            &mut store, cesium_sdk::ENTRYPOINT_NAME
        ).map_err(|_| VMError::WASMEntryPointNotFound)?;

        let packed_result: u64 = entry_point.call(
            &mut store,
            (0, context_bytes.len() as u32)
        ).map_err(|_| VMError::WASMFailedToCallFunction)?;

        let result = cesium_sdk::parse_response(packed_result, &store, &memory)
            .map_err(|_| VMError::WASMFailedToParseFunctionResponse)?;

        match result.status {
            cesium_sdk::STATUS_SUCCESS => {
                let is_reload_required = cesium_sdk::has_flag(result.flags, cesium_sdk::FLAG_RELOAD_REQUIRED);

                Ok(WorkerResult::Success {
                    logs: String::new(),
                    output: result.output,
                    is_reload_required,
                })
            },
            cesium_sdk::STATUS_FAILED => {
                let is_memory_corrupted = cesium_sdk::has_flag(result.flags, cesium_sdk::FLAG_MEMORY_CORRUPTED);
                let is_critical_error = cesium_sdk::has_flag(result.flags, cesium_sdk::FLAG_CRITICAL_ERROR);

                Ok(WorkerResult::Failed {
                    is_memory_corrupted,
                    is_critical_error,
                })
            }
            _ => Err(VMError::WASMInvalidStatusReturned)
        }
    }
}
