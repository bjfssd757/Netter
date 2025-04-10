use serde::{Deserialize, Serialize};

const STATE_FILE: &str = "SERVER_STATE.json";

#[derive(Serialize, Deserialize)]
pub struct ServerState {
    pub kind: String,
    pub pid: u32,
    pub host: String,
    pub port: u16,
}

pub fn save_state(kind: String, host: String, port: u16) {
    let state = ServerState {
        kind,
        pid: std::process::id(),
        host,
        port,
    };
    let serialized = serde_json::to_string(&state)
        .unwrap();
    std::fs::write(&STATE_FILE, serialized)
        .unwrap_or_else(|e| panic!("Failed to write in state file: {e}"));
}

pub fn load_state() -> Option<ServerState> {
    if let Ok(data) = std::fs::read_to_string(&STATE_FILE) {
        serde_json::from_str(&data).ok()
    } else {
        None
    }
}

pub fn delete_file() {
    std::fs::remove_file(&STATE_FILE).unwrap_or_else(|e| panic!("Failed to delete state file: {e}"));
}