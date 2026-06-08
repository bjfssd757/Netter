use std::cell::RefCell;

pub const ENTRYPOINT_NAME: &str         = "_cesium_entrypoint";

pub const STATUS_SUCCESS: u32           = 0x00;
pub const STATUS_FAILED: u32            = 0x01;

pub const FLAG_HAS_OUTPUT_DATA: u32     = 1 << 8;
pub const FLAG_LOGS_AVAILABLE: u32      = 1 << 9;
pub const FLAG_CRITICAL_ERROR: u32      = 1 << 10;
pub const FLAG_RELOAD_REQUIRED: u32     = 1 << 11;
pub const FLAG_MEMORY_CORRUPTED: u32    = 1 << 12;

struct WorkerLogStorage {
    worker_name: String,
    logs: Vec<String>,
}

thread_local! {
    static STORAGE: RefCell<WorkerLogStorage> = RefCell::new(WorkerLogStorage {
        worker_name: "unknown_worker".to_string(),
        logs: Vec::new(),
    });
}

pub fn init_worker_context(name: &str) {
    STORAGE.with(|storage| {
        let mut s = storage.borrow_mut();
        s.worker_name = name.to_string();
        s.logs.clear();
    });
}

pub fn log_push(message: &str) {
    STORAGE.with(|storage| {
        let mut s = storage.borrow_mut();
        let formatted_log = format!("[{}] {}", s.worker_name, message);
        s.logs.push(formatted_log);
    });
}

pub fn collect_logs_to_bytes() -> Vec<u8> {
    STORAGE.with(|storage| {
        let s = storage.borrow();
        if s.logs.is_empty() {
            return Vec::new();
        }
        let merged_str = s.logs.join("\n");
        merged_str.into_bytes()
    })
}

#[repr(C)]
pub struct WorkerResponseHeader {
    pub output_ptr: u32,
    pub output_len: u32,

    pub logs_ptr: u32,
    pub logs_len: u32,
}

pub const fn make_result_value(status: u32, flags: u32, response_ptr: u32) -> u64 {
    let packed = (status | flags) as u64;
    (packed << 32) | (response_ptr as u64)
}

/// Return (flags, status) from the value
pub const fn get_flags_and_status(value: u64) -> (u32, u32) {
    let flags = (value >> 32) as u32;
    let status = flags & 0xFF;

    (flags, status)
}

pub const fn get_response_ptr(value: u64) -> usize {
    (value & 0xFFFFFFFF) as usize
}

pub const fn has_flag(flags: u32, flag: u32) -> bool {
    (flags & flag) != 0
}

// =========================================================================
// Helpers
// =========================================================================

pub fn prepare_response(status: u32, mut flags: u32, output: Vec<u8>, logs: Vec<u8>) -> u64 {
    let mut output_ptr = 0u32;
    let mut output_len = 0u32;
    let mut logs_ptr = 0u32;
    let mut logs_len = 0u32;

    if !output.is_empty() {
        output_ptr = output.as_ptr() as u32;
        output_len = output.len() as u32;
        flags |= FLAG_HAS_OUTPUT_DATA;
        std::mem::forget(output);
    }

    if !logs.is_empty() {
        logs_ptr = logs.as_ptr() as u32;
        logs_len = logs.len() as u32;
        flags |= FLAG_LOGS_AVAILABLE;
        std::mem::forget(logs);
    }

    let header = Box::new(WorkerResponseHeader {
        output_ptr,
        output_len,
        logs_ptr,
        logs_len,
    });

    let header_ptr = Box::into_raw(header) as u32;

    make_result_value(status, flags, header_ptr)
}

#[derive(Debug, Clone)]
pub struct VmExecutionResult {
    pub status: u32,
    pub flags: u32,
    pub output: Vec<u8>,
    pub logs: Vec<u8>,
}

pub fn parse_response<T>(
    packed_value: u64,
    store: &wasmtime::Store<T>,
    memory: &wasmtime::Memory
) -> anyhow::Result<VmExecutionResult> {
    let (flags, status) = get_flags_and_status(packed_value);
    let header_ptr = get_response_ptr(packed_value);

    let mut output = Vec::new();
    let mut logs = Vec::new();

    if header_ptr != 0 {
        let header_size = size_of::<WorkerResponseHeader>();
        let mut header_bytes = vec![0u8; header_size];

        memory.read(store, header_ptr, &mut header_bytes)?;
        let header = unsafe { &*(header_bytes.as_ptr() as *const WorkerResponseHeader) };

        if has_flag(flags, FLAG_HAS_OUTPUT_DATA) && header.output_len > 0 {
            output = vec![0u8; header.output_len as usize];
            memory.read(store, header.output_ptr as usize, &mut output)?;
        }

        if has_flag(flags, FLAG_LOGS_AVAILABLE) && header.logs_len > 0 {
            logs = vec![0u8; header.logs_len as usize];
            memory.read(store, header.logs_ptr as usize, &mut logs)?;
        }
    }

    Ok(VmExecutionResult {
        status,
        flags,
        output,
        logs,
    })
}

// =========================================================================
// Traits
// =========================================================================


pub trait ContextToBytes {
    /// Serialize context to array of bytes.
    ///
    /// # Warning
    /// All numbers SHOULD serialize into Little Endian format
    fn to_bytes(&self) -> Vec<u8>;
}

impl ContextToBytes for i8 {
    fn to_bytes(&self) -> Vec<u8> { vec![*self as u8] }
}

impl ContextToBytes for u16 {
    fn to_bytes(&self) -> Vec<u8> { self.to_le_bytes().to_vec() }
}

impl ContextToBytes for i16 {
    fn to_bytes(&self) -> Vec<u8> { self.to_le_bytes().to_vec() }
}

impl ContextToBytes for u32 {
    fn to_bytes(&self) -> Vec<u8> { self.to_le_bytes().to_vec() }
}

impl ContextToBytes for i32 {
    fn to_bytes(&self) -> Vec<u8> { self.to_le_bytes().to_vec() }
}

impl ContextToBytes for u64 {
    fn to_bytes(&self) -> Vec<u8> { self.to_le_bytes().to_vec() }
}

impl ContextToBytes for i64 {
    fn to_bytes(&self) -> Vec<u8> { self.to_le_bytes().to_vec() }
}

// Реализация для чисел с плавающей точкой
impl ContextToBytes for f32 {
    fn to_bytes(&self) -> Vec<u8> { self.to_le_bytes().to_vec() }
}

impl ContextToBytes for f64 {
    fn to_bytes(&self) -> Vec<u8> { self.to_le_bytes().to_vec() }
}

// Реализация для логического типа (1 байт: 1 или 0)
impl ContextToBytes for bool {
    fn to_bytes(&self) -> Vec<u8> { vec![if *self { 1 } else { 0 }] }
}

// Реализация для строк и сырых векторов (они уже по сути являются контекстом)
impl ContextToBytes for String {
    fn to_bytes(&self) -> Vec<u8> { self.as_bytes().to_vec() }
}

impl ContextToBytes for &str {
    fn to_bytes(&self) -> Vec<u8> { self.as_bytes().to_vec() }
}

impl ContextToBytes for Vec<u8> {
    fn to_bytes(&self) -> Vec<u8> { self.clone() }
}

impl ContextToBytes for &[u8] {
    fn to_bytes(&self) -> Vec<u8> { self.to_vec() }
}