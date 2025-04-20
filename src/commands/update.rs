use std::process::Command;

pub fn update() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(&["pull"])
        .output()
        .map_err(|e| format!("Failed to execute git pull: {e}"))?;

    if !output.stdout.is_empty() {
        println!("{}", std::str::from_utf8(&output.stdout)?);
    }

    if !output.stderr.is_empty() {
        eprintln!("{}", std::str::from_utf8(&output.stderr)?);
    }

    Ok(())
}