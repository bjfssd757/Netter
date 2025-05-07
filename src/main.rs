use clap::{Parser, Subcommand};
use std::{path::{Path, PathBuf}, process::ExitCode};
use netter_logger;
use log::{
    info,
    error,
    trace,
    debug,
    warn,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
#[cfg(unix)]
use tokio::net::UnixStream;

use netter_core::{
    Command,
    Response,
    CoreError,
    ConfigSource,
    ServerInfo,
};

#[cfg(windows)]
const IPC_PATH: &str = r"\\.\pipe\MyNetterServicePipe";
#[cfg(unix)]
const IPC_PATH: &str = "/run/netterservice/netterd.sock";

const CLI_LOG_DIR: &str = "logs_cli";

#[derive(Parser, Debug)]
#[command(name = "netter")]
#[command(author = "bjfssd757")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Netter CLI - Утилита для управления HTTP серверами Netter через службу.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short = 'v', long = "verbose", global = true, help = "Включить подробный вывод (verbose)")]
    verbose: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Ping,
    Start {
        #[arg(short, long)]
        config: String,
    },
    Stop {
        #[arg(short, long)]
        id: String,
    },
    Status {
        #[arg(short, long)]
        id: String,
    },
    List,
    Update,
    Install,
    Uninstall,
    ServiceStart,
    ServiceStop,
    ServiceStatus,
}


#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let v = cli.verbose.clone();
    if v {
        if let Err(e) = std::fs::create_dir_all(CLI_LOG_DIR) {
            eprintln!("CRITICAL: Failed to create CLI log directory '{}': {}", CLI_LOG_DIR, e);
            return ExitCode::FAILURE;
        }
        let log_level = log::LevelFilter::Debug;
        if let Err(e) = netter_logger::init(Some(CLI_LOG_DIR), log_level, log::LevelFilter::Trace) {
            eprintln!("CRITICAL: Failed to initialize CLI logger: {}", e);
            return ExitCode::FAILURE;
        }
    } else {
        if let Err(e) = netter_logger::init(None::<PathBuf>, log::LevelFilter::Info, log::LevelFilter::Error) {
            eprintln!("CRITICAL: Failed to initialize CLI logger: {}", e);
            return ExitCode::FAILURE;
        }
    }
    info!("Netter CLI v{} started.", env!("CARGO_PKG_VERSION"));
    debug!("CLI Arguments: {:?}", cli);

    match cli.command.clone() {
        Commands::Install => {
            match install_service().await {
                Ok(code) => {
                    return code
                },
                Err(_) => {
                    return ExitCode::FAILURE
                }
            }
        },
        Commands::Uninstall => {
            match uninstall_service().await {
                Ok(code) => {
                    return code
                },
                Err(_) => {
                    return ExitCode::FAILURE
                }
            }
        },
        Commands::ServiceStart => {
            match start_service().await {
                Ok(code) => {
                    return code
                },
                Err(_) => {
                    return ExitCode::FAILURE
                }
            }
        },
        Commands::ServiceStop => {
            match stop_service().await {
                Ok(code) => {
                    return code
                },
                Err(_) => {
                    return ExitCode::FAILURE
                }
            }
        },
        Commands::ServiceStatus => {
            match query_service_status().await {
                Ok(code) => {
                    return code
                },
                Err(_) => {
                    return ExitCode::FAILURE
                }
            }
        },
        _ => {}
    }

    let command_to_send = match create_service_command(cli.command.clone()).await {
        Ok(cmd) => cmd,
        Err(e) => { error!("Error preparing command: {}", e); return ExitCode::FAILURE; }
    };

    info!("Sending command to service: {:?}", command_to_send);
    match send_command_to_service(command_to_send).await {
        Ok(response) => {
            info!("Response received from service.");
            debug!("Raw response: {:?}", response);
            handle_service_response(response.clone());
            if matches!(response, Response::Error(_)) { ExitCode::FAILURE } else { ExitCode::SUCCESS }
        }
        Err(e) => {
            error!("Communication error with service: {}", e);
            eprintln!("\nError: Could not connect to the Netter service.");
            #[cfg(windows)] { eprintln!("You can check its status using 'sc.exe query NetterService' or start it with 'sc.exe start NetterService'."); }
            #[cfg(unix)] { eprintln!("You can check its status using 'systemctl status netterd' or start it with 'systemctl start netterd'."); }
            eprintln!("Details: {}", e);
            eprintln!("Details: {}", e);
            ExitCode::FAILURE
        }
    }
}

async fn create_service_command(command: Commands) -> Result<Command, Box<dyn std::error::Error>> {
    match command {
        Commands::Ping => Ok(Command::Ping),
        Commands::Start { config: path } => {
            info!("Reading configuration file: {}", path);
            if !Path::new(&path).extension().map_or(false, |ext| ext.eq_ignore_ascii_case("rd")) {
                let err_msg = format!("Unsupported configuration file extension: '{}'. Only '.rd' files are supported.", path);
                error!("{}", err_msg);
                return Err(err_msg.into());
            }
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    info!("Configuration type determined: Custom Language (.rd)");
                    Ok(Command::StartServer {
                        config: ConfigSource::CustomLangFileContent(content)
                    })
                }
                Err(e) => {
                    let err_msg = format!("Failed to read configuration file '{}': {}", path, e);
                    error!("{}", err_msg);
                    Err(err_msg.into())
                }
            }
        }
        Commands::Stop { id } => Ok(Command::StopServer { server_id: id }),
        Commands::Status { id } => Ok(Command::GetServerStatus { server_id: id }),
        Commands::List => {
            info!("Preparing List command (GetAllServersStatus)");
            Ok(Command::GetAllServersStatus)
        }
        Commands::Update => Ok(Command::CheckForUpdate),
        Commands::Install | Commands::Uninstall | Commands::ServiceStart | Commands::ServiceStop | Commands::ServiceStatus => {
            unreachable!("Management commands should be handled before create_service_command")
        }
    }
}


async fn send_command_to_service(command: Command) -> Result<Response, Box<dyn std::error::Error>> {
    trace!("Attempting to connect to IPC: {}", IPC_PATH);

    #[cfg(windows)]
    let mut stream = ClientOptions::new()
        .open(IPC_PATH)
        .map_err(|e| format!("Failed to open pipe '{}': {}", IPC_PATH, e))?;
    #[cfg(unix)]
    let mut stream = UnixStream::connect(IPC_PATH)
        .await
        .map_err(|e| format!("Failed to connect to socket '{}': {}", IPC_PATH, e))?;

    trace!("Successfully connected to IPC.");

    let encoded_command = bincode::serialize(&command)?;
    trace!("Serialized command ({} bytes)", encoded_command.len());

    let command_size = encoded_command.len() as u32;
    stream.write_all(&command_size.to_be_bytes()).await?;
    stream.write_all(&encoded_command).await?;
    stream.flush().await?;

    trace!("Command sent to service/daemon. Awaiting response...");

    let mut size_buf = [0u8; 4];
    stream.read_exact(&mut size_buf).await
        .map_err(|e| format!("Failed to read response size header: {}", e))?;
    let response_size = u32::from_be_bytes(size_buf) as usize;
    trace!("Response size header indicates {} bytes.", response_size);

    if response_size > 10 * 1024 * 1024 {
        return Err(format!("Response size {} exceeds limit", response_size).into());
    }

    let mut response_buffer = vec![0u8; response_size];
    stream.read_exact(&mut response_buffer).await
         .map_err(|e| format!("Failed to read response body (expected {} bytes): {}", response_size, e))?;

    trace!("Response received ({} bytes).", response_buffer.len());

    let response: Response = bincode::deserialize(&response_buffer)?;
    trace!("Deserialized response: {:?}", response);

    Ok(response)
}

fn handle_service_response(response: Response) {
    println!("--- Netter Service Response ---");
    match response {
        Response::Pong => {
            println!("Status: Pong!");
            println!("Netter service is available.");
        }
        Response::Ok => {
            println!("Status: Success!");
            println!("Operation completed successfully by the service.");
        }
        Response::ServerStarted(info) => {
            println!("Status: Server Started!");
            print_server_info(&info);
            println!("Use ID '{}' for stop/status commands.", info.server_id);
        }
        Response::ServerStopped(server_id) => {
            println!("Status: Server Stopped.");
            println!("  Stopped Server ID: {}", server_id);
        }
        Response::ServerStatus(info) => {
            println!("Status: Server Information");
            print_server_info(&info);
        }
        Response::AllServersStatusReport(servers) => {
            println!("Status: All Servers Information");
            if servers.is_empty() {
                println!("Status: No active servers managed by the service.");
            } else {
                println!("Status: List of active servers ({})", servers.len());
                for info in servers {
                    println!("---");
                    print_server_info(&info);
                }
                println!("---");
            }
        }
        Response::UpdateAvailable(info) => {
            println!("Status: Update Available!");
            println!("  Current Version: {}", info.current_version);
            println!("  Latest Version:  {}", info.latest_version);
            println!("  Download URLs:");
            if info.download_urls.is_empty() { println!("    (URLs not found in release)"); }
            else { for (artifact, url) in info.download_urls { println!("    - {}: {}", artifact, url); } }
            println!("\nSee documentation for update instructions.");
        }
        Response::UpToDate(version) => {
            println!("Status: Application is up-to-date.");
            println!("  Current Version: {}", version);
        }
        Response::Error(core_error) => {
            println!("Status: Error!");
            error!("Service returned error: {}", core_error);
            match core_error {
                CoreError::ServerNotFound(id) => eprintln!("Error: Server with ID '{}' not found by service.", id),
                CoreError::ConfigParseError(msg) => eprintln!("Configuration Error: {}", msg),
                CoreError::IoError(msg) => eprintln!("Service I/O Error: {}", msg),
                CoreError::OperationFailed(msg) => eprintln!("Service operation failed: {}", msg),
                CoreError::InvalidInput(msg) => eprintln!("Invalid input provided: {}", msg),
                _ => eprintln!("An error occurred while the service executed the command: {}", core_error),
            }
            eprintln!("Check CLI logs ({}) and service logs for more details.", CLI_LOG_DIR);
        }
    }
    println!("---------------------------");
}

fn print_server_info(info: &ServerInfo) {
    println!("  ID:      {}", info.server_id);
    println!("  Type:    {:?}", info.server_type);
    println!("  Address: {}", info.address);
    println!("  Status:  {}", info.status);
    if let Some(pid) = info.pid { println!("  PID:     {}", pid); }
}

async fn install_service() -> Result<ExitCode, Box<dyn std::error::Error>> {
    info!("Attempting to install Netter service...");
    #[cfg(windows)]
    {
        println!("(Windows) Running 'sc create'...");
        let current_exe = std::env::current_exe()?;
        let service_exe = current_exe.parent().ok_or("Cannot find parent directory")?.join("netter_service.exe");
        if !service_exe.exists() {
            return Err(format!("netter_service.exe not found in the same directory as the CLI: {}", service_exe.display()).into());
        }
        let service_path_str = service_exe.to_str().ok_or("Invalid service path encoding")?;

        let output = std::process::Command::new("sc.exe")
            .args([
                "create",
                "NetterService",
                &format!("binPath={}", service_path_str),
                "start=auto",
                "DisplayName=Netter Service",
            ])
            .output()?;

        if output.status.success() {
            println!("Service created successfully.");
            println!("You may need to configure firewall rules if servers listen on non-local addresses.");
            println!("Use 'netter service start' to start the service.");
            Ok(ExitCode::SUCCESS)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("sc create failed: {}", stderr);
            Err(format!("Failed to create service: {}", stderr).into())
        }
    }
    #[cfg(unix)]
    {
        let info = os_info::get();
        let type_os = info.os_type().to_string().to_lowercase();
    
        match type_os.as_str() {
            "devuan" | "slackware" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/init.d/netterd";
                distributions_pathes(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "alpine" | "gentoo" | "artix with openrc" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/init.d/netterd";
                distributions_pathes(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "void" | "artix with runit" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/sv/netterd/run";
                distributions_pathes(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "artix with s6" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/s6/sv/netterd/run";
                distributions_pathes(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "tinycore" | "minimal alpine" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/init.d/netterd";
                distributions_pathes(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "nixos" => {
                println!("For NixOS, services must be defined in the system configuration.");
            }
            _ => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/systemd/system/netterd.service";
                let _ = distributions_pathes(SERVICE_DEST_STR, UNIT_FILE_PATH).await;
            }
        }

    }
    #[cfg(not(any(windows, unix)))]
    {
        Err("Unsupported OS for service installation.".into())
    }
    Ok(ExitCode::SUCCESS)
}

async fn distributions_pathes(service_dest_str: &str, unit_file_path: &str) -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("(Unix) This command requires root privileges (sudo).");
    println!("Attempting basic installation (copy executable and create systemd unit)...");

    let current_cli_exe = std::env::current_exe()?;
    let service_exe_source = current_cli_exe
        .parent()
        .ok_or("Cannot find parent directory of CLI")?
        .join("netter_service");

    if !service_exe_source.exists() {
        return Err(format!(
            "Daemon executable 'netter_service' not found in the directory: {}",
            service_exe_source
                .parent()
                .unwrap_or(Path::new("."))
                .display()
        )
        .into());
    }
    let service_source_str = service_exe_source
        .to_str()
        .ok_or("Invalid source path encoding")?;

    run_os_command(
        "sudo",
        &["cp", "-v", service_source_str, service_dest_str],
        "copy daemon executable",
    )?;
    run_os_command("sudo", &["chmod", "+x", service_dest_str], "make executable")?;

    if !is_systemd_running() {
        warn!("Systemd not detected on this system!");
        println!(
            "\nDaemon executable copied to {}.",
            service_dest_str.clone()
        );
        println!("Automatic service managment is not available.");
        println!("You need to manage the 'netterd' process manually using your system's init system (e.g., SysVinit, OpenRC) or run it in the background (e.g., 'nohup {} &').", service_dest_str);
    }

    let unit_content = format!(
                    r#"[Unit]
        Description=Netter Service Daemon
        After=network.target

        [Service]
        Type=simple
        ExecStart={}
        WorkingDirectory={}
        Restart=on-failure
        User=root
        Group=root
        StandardOutput=journal
        StandardError=journal

        [Install]
        WantedBy=multi-user.target
        "#,
        service_dest_str,
        Path::new(service_dest_str)
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
    );


    println!("Creating systemd unit file at {}...", unit_file_path);
    let echo_cmd = std::process::Command::new("echo")
        .arg(unit_content)
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    let tee_cmd = std::process::Command::new("sudo")
        .arg("tee")
        .arg(unit_file_path)
        .stdin(echo_cmd.stdout.ok_or("Failed to pipe echo output")?)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()?;

    if !tee_cmd.success() {
        return Err(format!("Failed to write systemd unit file '{}'", unit_file_path).into());
    }
    println!("Systemd unit file created.");

    run_os_command("sudo", &["systemctl", "daemon-reload"], "reload systemd")?;
    run_os_command("sudo", &["systemctl", "enable", "netterd"], "enable daemon")?;

    println!("\nDaemon 'netterd' installed and enabled successfully.");
    println!("Use 'netter service-start' to start the daemon.");

    Ok(ExitCode::SUCCESS)
}

async fn uninstall_service() -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("Attempting to uninstall Netter service/daemon...");
   #[cfg(windows)]
   {  
       println!("(Windows) Stopping and deleting service 'NetterService'...");
       let _ = std::process::Command::new("sc").args(["stop", "NetterService"]).output()?;
       let output = std::process::Command::new("sc").args(["delete", "NetterService"]).output()?;
       if output.status.success() {
           println!("Service 'NetterService' deleted successfully.");
           Ok(ExitCode::SUCCESS)
       } else {
           let stderr = String::from_utf8_lossy(&output.stderr);
            
           if stderr.contains("does not exist") || stderr.contains("не существует") {
               println!("Service 'NetterService' was not found (already uninstalled?).");
               Ok(ExitCode::SUCCESS)
           } else {
               error!("sc delete failed: {}", stderr);
               Err(format!("Failed to delete service: {}", stderr).into())
           }
       }
   }
   #[cfg(unix)]
   {
        let info = os_info::get();
        let type_os = info.os_type().to_string().to_lowercase();

        match type_os.as_str() {
            "devuan" | "slackware" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/init.d/netterd";
                uninstall_netterd_service(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "alpine" | "gentoo" | "artix with openrc" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/init.d/netterd";
                uninstall_netterd_service(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "void" | "artix with runit" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/sv/netterd/run";
                uninstall_netterd_service(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "artix with s6" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/s6/sv/netterd/run";
                uninstall_netterd_service(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "tinycore" | "minimal alpine" => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/init.d/netterd";
                uninstall_netterd_service(SERVICE_DEST_STR, UNIT_FILE_PATH).await?;
            }
            "nixos" => {
                println!("For NixOS, services must be defined in the system configuration.");
            }
            _ => {
                const SERVICE_DEST_STR: &str = "/usr/local/bin/netterd";
                const UNIT_FILE_PATH: &str = "/etc/systemd/system/netterd.service";
                let _ = uninstall_netterd_service(SERVICE_DEST_STR, UNIT_FILE_PATH).await;
            }
        }
   }
   #[cfg(not(any(windows, unix)))]
   {
       Err("Service uninstallation is not supported on this OS.".into())
   }
   Ok(ExitCode::SUCCESS)
}

async fn uninstall_netterd_service(service_dest_str: &str, unit_file_path: &str) -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("(Unix) This command requires root privileges (sudo).");

    if is_systemd_running() {
        println!("Systemd detected. Disabling and stopping 'netterd' service...");
        let _ = run_os_command("sudo", &["systemctl", "disable", service_dest_str], "disable daemon");
        let _ = run_os_command("sudo", &["systemctl", "stop", service_dest_str], "stop daemon");
    } else if is_sysvinit_running() {
        println!("Sysvinit detected. Disabling and stopping 'netterd' service...");
        let init_path = format!("/etc/init.d/{}", service_dest_str);
        let _ = run_os_command("sudo", &[&init_path, "stop"], "stop daemon");
        let _ = run_os_command("sudo", &["update-rc.d", "-f", service_dest_str, "remove"], "disable daemon");
    } else if is_openrc_running() {
        println!("Openrc detected. Disabling and stopping 'netterd' service...");
        let _ = run_os_command("sudo", &["rc-service", service_dest_str, "stop"], "stop daemon");
        let _ = run_os_command("sudo", &["rc-update", "del", service_dest_str], "disable daemon");
    } else if is_runit_running() {
        println!("Runit detected. Disabling and stopping 'netterd' service...");
        let init_path = format!("/etc/sv/{}", service_dest_str);
        let _ = run_os_command("sudo", &["sv", "down", service_dest_str], "stop daemon");
        let _ = run_os_command("sudo", &["rm", "-rf", &init_path], "disable daemon");
    } else if is_s6_running() {
        println!("R6 detected. Disabling and stopping 'netterd' service...");
        let init_path = format!("/etc/s6/sv/{}", service_dest_str);
        let _ = run_os_command("sudo", &["s6-svc", "-d", &init_path], "stop daemon");
        let _ = run_os_command("sudo", &["rm", "-rf", &init_path], "disable daemon");
    } else if is_tinycore_running() {
        println!("Tinycore detected. Disabling and stopping 'netterd' service...");
        let init_path = format!("/etc/init.d/{}", service_dest_str);
        let _ = run_os_command("sudo", &[&init_path, "stop"], "stop daemon");
        let _ = run_os_command("sudo", &["update-rc.d", "-f", service_dest_str, "remove"], "disable daemon");
    } else if is_nixos_running() {
        println!("Please remove 'netterd' from your NixOS configuration and run 'sudo nixos-rebuild switch' to stop and disable the service.");            
    } else {
        warn!("System initialization not detected. Skipping disable/stop.");
        println!("If the daemon is running, you need to stop the 'netterd' process manually (e.g., using 'pkill netterd').");
    }

    /* first path */
    let unit_file_dest = Path::new(unit_file_path);
    if unit_file_dest.exists() {
        run_os_command(
            "sudo",
            &["rm", "-v", unit_file_dest.to_str().unwrap()],
            "remove systemd unit file",
        )?;
    } else {
        println!("Systemd unit file not found (already removed or not installed via systemd?).");
    }

    /* second path */
    let daemon_dest = Path::new(service_dest_str);
    if daemon_dest.exists() {
        run_os_command(
            "sudo",
            &["rm", "-v", daemon_dest.to_str().unwrap()],
            "remove daemon executable",
        )?;
    } else {
        println!("Daemon executable not found (already removed?).");
    }

    if is_systemd_running() {
        run_os_command("sudo", &["systemctl", "daemon-reload"], "reload systemd")?;
        run_os_command(
            "sudo",
            &["systemctl", "reset-failed"],
            "reset failed units",
        )?;
    }

    println!("\nDaemon 'netterd' uninstalled successfully.");
    println!(
        "You might want to manually remove state files ({}) and log files.",
        CLI_LOG_DIR
    );
    Ok(ExitCode::SUCCESS)
}

async fn start_service() -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("Attempting to start Netter service/daemon...");
   #[cfg(windows)] {
       run_os_command("sc", &["start", "NetterService"], "start service")
   }
   #[cfg(unix)] {
        println!("Attempting to start Netter service/daemon...");
        println!("(Unix) This command may require root privileges (sudo).");

        if is_systemd_running() {
            run_os_command("sudo", &["systemctl", "start", "netterd"], "start daemon")
        } else if is_sysvinit_running() {
            run_os_command("sudo", &["/etc/init.d/netterd", "start"], "start daemon")
        } else if is_openrc_running() {
            run_os_command("sudo", &["rc-service", "netterd", "start"], "start daemon")
        } else if is_runit_running() {
            run_os_command("sudo", &["sv", "up", "netterd"], "start daemon")
        } else if is_s6_running() {
            run_os_command("sudo", &["s6-svc", "-u", "/run/service/netterd"], "start daemon")
        } else if is_tinycore_running() {
            run_os_command("sudo", &["/etc/init.d/netterd", "start"], "start daemon")
        } else if is_nixos_running() {
            println!("On NixOS, please enable and start the service via configuration.nix.");
            Ok(ExitCode::SUCCESS)
        } else {
            warn!("No supported init system detected. Manual start may be required.");
            Err("Unsupported init system for starting the service.".into())
        } 
   }
   #[cfg(not(any(windows, unix)))]
   { Err("Service start is not supported on this OS.".into()) }
}

async fn stop_service() -> Result<ExitCode, Box<dyn std::error::Error>> {
   println!("Attempting to stop Netter service/daemon...");
   #[cfg(windows)] {
       run_os_command("sc", &["stop", "NetterService"], "stop service")
   }
   #[cfg(unix)]
   {
       println!("(Unix) This command may require root privileges (sudo).");
       let info = os_info::get();
       let type_os = info.os_type().to_string().to_lowercase();

       match type_os.as_str() {
           "devuan" | "slackware" | "tinycore" | "minimal alpine" => {
               run_os_command("sudo", &["/etc/init.d/netterd", "stop"], "stop daemon")?;
           }
           "alpine" | "gentoo" | "artix with openrc" => {
               run_os_command("sudo", &["rc-service", "netterd", "stop"], "stop daemon")?;
           }
           "void" | "artix with runit" => {
               run_os_command("sudo", &["sv", "down", "netterd"], "stop daemon")?;
           }
           "artix with s6" => {
               run_os_command("sudo", &["s6-svc", "-d", "/run/service/netterd"], "stop daemon")?;
           }
           "nixos" => {
               println!("On NixOS, please remove or disable the service via configuration.nix.");
           }
           _ => {
               run_os_command("sudo", &["systemctl", "stop", "netterd"], "stop daemon")?;
           }
       }

       Ok(ExitCode::SUCCESS)
   }
   
   #[cfg(not(any(windows, unix)))]
   { Err("Service stop is not supported on this OS.".into()) }
}

async fn query_service_status() -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("Querying Netter service/daemon status...");
     #[cfg(windows)] {
        match run_os_command("sc", &["query", "NetterService"], "query service status") {
            Ok(_) => Ok(ExitCode::SUCCESS),
            Err(e) => Err(format!("{e}").into())
        }
    }
    #[cfg(unix)] {  
        
        println!("> systemctl status netterd");
        let status = std::process::Command::new("systemctl")
            .args(["status", "netterd"])
            .status()?;
        println!(
            "(Command finished with status: {})",
            status.code().unwrap_or(-1)
        );
        Ok(ExitCode::SUCCESS)
    }
    #[cfg(not(any(windows, unix)))]
    { Err("Service status query is not supported on this OS.".into()) }
}

#[cfg(unix)]
fn run_command(command: &str, args: &[&str]) -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("> {} {}", command, args.join(" "));
    let status = std::process::Command::new(command).args(args).status()?;
    if status.success() {
        Ok(ExitCode::SUCCESS)
    } else {
        Err(format!("Command '{} {}' failed with status: {}", command, args.join(" "), status).into())
    }
}

fn run_os_command(command: &str, args: &[&str], action_desc: &str) -> Result<ExitCode, Box<dyn std::error::Error>> {
     println!("> {} {}", command, args.join(" "));
     let output = std::process::Command::new(command).args(args).output()?;

     if output.status.success() {
         println!("Successfully executed {}.", action_desc);
         if !output.stdout.is_empty() {
             println!("--- Output ---");
             println!("{}", String::from_utf8_lossy(&output.stdout));
             println!("--------------");
         }
         Ok(ExitCode::SUCCESS)
     } else {
         let stderr = String::from_utf8_lossy(&output.stderr);
         error!("Failed to {}: {}", action_desc, stderr);
         Err(format!("Failed to {}: {}", action_desc, stderr).into())
     }
}

#[cfg(unix)]                     
fn is_systemd_running() -> bool {
    Path::new("/run/systemd/system").exists()
}

#[cfg(unix)]
fn is_sysvinit_running() -> bool {
    Path::new("/etc/init.d").exists()
}

#[cfg(unix)]
fn is_openrc_running() -> bool {
    Path::new("/etc/init.d").exists() && Path::new("/run/openrc").exists()
}

#[cfg(unix)]
fn is_runit_running() -> bool {
    Path::new("/etc/runit").exists() || Path::new("/run/runit").exists()
}

#[cfg(unix)]
fn is_s6_running() -> bool {
    Path::new("/etc/s6").exists() || Path::new("/run/s6").exists()
}

#[cfg(unix)]
fn is_tinycore_running() -> bool {
    Path::new("/etc/init.d").exists()
}

#[cfg(unix)]
fn is_nixos_running() -> bool {
    Path::new("/etc/nixos").exists()
}