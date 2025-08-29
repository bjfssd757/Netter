use netter_plugger::{netter_plugin, generate_dispatch_func};
use rand::Rng;

generate_dispatch_func!();

// ----------- network -----------

#[netter_plugin]
fn is_email_valid(email: String) -> Result<String, String> {
    if email.contains('@') && email.contains('.') {
        Ok(true.to_string())
    } else {
        Ok(false.to_string())
    }
}

#[netter_plugin]
fn is_ip_valid(ip: String) -> Result<String, String> {
    let (ip_part, port_part) = if let Some(idx) = ip.find(':') {
        let (ip_p, port_p) = ip.split_at(idx);
        (ip_p, Some(&port_p[1..]))
    } else {
        (ip.as_str(), None)
    };

    let nums: Vec<&str> = ip_part.split('.').collect();
    if nums.len() != 4 {
        return Ok(false.to_string());
    }
    for n in &nums {
        let _n: u8 = match n.parse() {
            Ok(val) => val,
            Err(_) => return Ok(false.to_string()),
        };
    }

    if let Some(port_str) = port_part {
        let _port: u16 = match port_str.parse() {
            Ok(val) => val,
            Err(_) => return Ok(false.to_string()),
        };
    }

    Ok(true.to_string())
}

// ----------- env -----------

#[netter_plugin]
fn env_var(var: String) -> Result<String, String> {
    match std::env::var(&var) {
        Ok(val) => Ok(val),
        Err(_) => Err(format!("Environment variable {} not found", var))
    }
}

// ----------- math -----------

#[netter_plugin]
fn random(min: i32, max: i32) -> Result<String, String> {
    let mut rng = rand::rng();
    let res = rng.random_range(min..=max);

    Ok(res.to_string())
}

// ----------- string -----------

#[netter_plugin]
fn to_uppercase(str: String) -> Result<String, String> {
    Ok(str.to_uppercase())
}

#[netter_plugin]
fn to_lowercase(str: String) -> Result<String, String> {
    Ok(str.to_lowercase())
}

// ----------- time -----------

#[netter_plugin]
fn sleep(duration: i32) -> Result<String, String> {
    std::thread::sleep(std::time::Duration::from_secs(duration as u64));
    Ok(String::from(""))
}

#[netter_plugin]
fn now() -> Result<String, String> {
    let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(n) => n.as_secs() as i32,
        Err(_) => Err("Failed to get system time")?,
    };
    Ok(now.to_string())
}
