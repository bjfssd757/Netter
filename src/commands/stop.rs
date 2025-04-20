use std::process::Command;
use crate::state::delete_file;
use log::info;

pub async fn stop(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        let output = Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .output()
            .map_err(|e| format!("Failed to execute taskkill: {e}"))?;

        if output.status.success() {
            delete_file()?;
            
            info!("Process stopped successfully");
            Ok(())
        } else {
            Err(Box::<dyn std::error::Error>::from(
                format!("Failed to stop process: {}", String::from_utf8_lossy(&output.stderr))))
        }
    } else if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        let output = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()
            .map_err(|e| format!("Failed to execute kill: {e}"))?;

        if output.status.success() {
            delete_file()?;

            info!("Process with PID {} stopped successfully.", pid);
            Ok(())
        } else {
            Err(Box::<dyn std::error::Error>::from(
                format!("Failed to stop process with PID {}: {}",
                pid,
                String::from_utf8_lossy(&output.stderr))))
        }
    } else {
        Err(Box::<dyn std::error::Error>::from(
            "Unsupported OS"))
    }
}