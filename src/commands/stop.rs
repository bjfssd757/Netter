use std::process::Command;
use crate::state::delete_file;

pub async fn stop(pid: u32) {
    if cfg!(target_os = "windows") {
        let output = Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .output()
            .unwrap_or_else(|e| panic!("Failed to execute taskkill: {e}"));

        if output.status.success() {
            println!("Process stopped successfully");

            delete_file();
        } else {
            panic!("Failed to stop process: {}", String::from_utf8_lossy(&output.stderr));
        }
    } else if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        let output = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()
            .expect("Failed to execute kill");

        if output.status.success() {
            println!("Process with PID {} stopped successfully.", pid);
        } else {
            panic!(
                "Failed to stop process with PID {}: {}",
                pid,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    } else {
        panic!("Unsupported OS");
    }
}