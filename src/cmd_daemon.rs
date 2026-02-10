//! Daemon subcommand handlers for AutoHands.

use std::path::PathBuf;

use tracing::{error, info, warn};

use autohands_daemon::{Daemon, DaemonConfig, DaemonError};

use crate::adapters::default_pid_file;
use crate::cli::DaemonAction;

/// Handle daemon subcommands.
pub(crate) async fn handle_daemon_command(
    action: DaemonAction,
    work_dir: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        DaemonAction::Start { foreground, pid_file } => {
            daemon_start(work_dir, foreground, pid_file).await
        }
        DaemonAction::Stop { pid_file, force } => {
            daemon_stop(pid_file, force).await
        }
        DaemonAction::Restart { pid_file } => {
            daemon_restart(work_dir, pid_file).await
        }
        DaemonAction::Status { pid_file } => {
            daemon_status(pid_file).await
        }
        DaemonAction::Install { label, system } => {
            daemon_install(&label, system).await
        }
        DaemonAction::Uninstall { label, system } => {
            daemon_uninstall(&label, system).await
        }
        DaemonAction::Logs { lines, label } => {
            daemon_logs(&label, lines).await
        }
    }
}

/// Start the daemon.
async fn daemon_start(
    work_dir: PathBuf,
    foreground: bool,
    pid_file: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pid_path = pid_file.unwrap_or_else(default_pid_file);

    // Ensure parent directory exists
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let config = DaemonConfig {
        pid_file: pid_path.clone(),
        daemonize: !foreground,
        work_dir: Some(work_dir.clone()),
        auto_restart: true,
        max_restarts: 10,
        ..Default::default()
    };

    let daemon = Daemon::new(config)?;

    // Check if already running
    if let Some(pid) = daemon.get_running_pid().await? {
        error!("Daemon already running with PID {}", pid);
        return Err(Box::new(DaemonError::AlreadyRunning {
            path: pid_path,
            pid,
        }));
    }

    info!("Starting daemon...");

    // Run the daemon with the main server function
    daemon
        .run(|| async {
            // Initialize kernel
            let _kernel = autohands_core::Kernel::new(work_dir.clone());
            info!("Kernel initialized");

            // TODO: Start actual server
            info!("AutoHands daemon ready");

            // Keep running until shutdown signal
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        })
        .await?;

    Ok(())
}

/// Stop the daemon.
async fn daemon_stop(
    pid_file: Option<PathBuf>,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pid_path = pid_file.unwrap_or_else(default_pid_file);

    let config = DaemonConfig {
        pid_file: pid_path.clone(),
        ..Default::default()
    };

    let daemon = Daemon::new(config)?;

    match daemon.get_running_pid().await? {
        Some(pid) => {
            info!("Stopping daemon (PID: {})...", pid);

            // Send SIGTERM
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;

                let signal = if force { Signal::SIGKILL } else { Signal::SIGTERM };
                kill(Pid::from_raw(pid as i32), signal)
                    .map_err(|e| format!("Failed to send signal: {}", e))?;
            }

            #[cfg(not(unix))]
            {
                warn!("Signal sending not supported on this platform");
            }

            // Wait for process to exit
            for _ in 0..30 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if !autohands_daemon::PidFile::is_process_running(pid) {
                    info!("Daemon stopped");
                    // Clean up PID file
                    let _ = std::fs::remove_file(&pid_path);
                    return Ok(());
                }
            }

            if force {
                error!("Daemon did not stop in time");
            } else {
                warn!("Daemon did not stop gracefully, try --force");
            }
        }
        None => {
            info!("Daemon is not running");
        }
    }

    Ok(())
}

/// Restart the daemon.
async fn daemon_restart(
    work_dir: PathBuf,
    pid_file: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Restarting daemon...");

    // Stop first
    daemon_stop(pid_file.clone(), false).await?;

    // Wait a bit
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Start
    daemon_start(work_dir, false, pid_file).await
}

/// Get daemon status.
async fn daemon_status(pid_file: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let pid_path = pid_file.unwrap_or_else(default_pid_file);

    let config = DaemonConfig {
        pid_file: pid_path.clone(),
        ..Default::default()
    };

    let daemon = Daemon::new(config)?;
    let status = daemon.status().await;

    println!("AutoHands Daemon Status");
    println!("=======================");
    println!("PID File: {}", pid_path.display());
    println!("{}", status);

    if let Some(pid) = status.pid {
        println!("\nDaemon is RUNNING (PID: {})", pid);
    } else {
        println!("\nDaemon is NOT RUNNING");
    }

    Ok(())
}

/// Install as system service.
async fn daemon_install(label: &str, _system: bool) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        use autohands_daemon::launchd::{LaunchAgent, LaunchAgentConfig};

        let exe_path = std::env::current_exe()?;
        let config = LaunchAgentConfig::with_label(label)
            .program(exe_path)
            .program_arguments(vec![
                "daemon".to_string(),
                "start".to_string(),
                "--foreground".to_string(),
            ]);

        let agent = LaunchAgent::new(config);

        if agent.is_installed() {
            warn!("LaunchAgent already installed, updating...");
            agent.uninstall()?;
        }

        agent.install()?;
        info!("Successfully installed LaunchAgent: {}", label);
        println!("\nAutoHands installed as macOS LaunchAgent");
        println!("Service will start automatically on login");
        println!("\nManual control:");
        println!("  Start:  launchctl start {}", label);
        println!("  Stop:   launchctl stop {}", label);
        println!("  Status: launchctl list | grep {}", label);
        println!("\nLogs:");
        println!("  stdout: ~/.autohands/logs/stdout.log");
        println!("  stderr: ~/.autohands/logs/stderr.log");

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use autohands_daemon::systemd::{SystemdService, SystemdConfig};

        let exe_path = std::env::current_exe()?;
        let service_name = label.replace("com.", "").replace('.', "-");

        let mut config = SystemdConfig::with_name(&service_name)
            .exec_start(exe_path)
            .exec_args(vec![
                "daemon".to_string(),
                "start".to_string(),
                "--foreground".to_string(),
            ]);

        if _system {
            config = config.system_mode();
        } else {
            config = config.user_mode();
        }

        let service = SystemdService::new(config);

        if service.is_installed() {
            warn!("Systemd service already installed, updating...");
            service.uninstall()?;
        }

        service.install()?;
        info!("Successfully installed Systemd service: {}", service_name);

        let mode = if _system { "system" } else { "user" };
        let user_flag = if _system { "" } else { "--user " };

        println!("\nAutoHands installed as Linux Systemd {} service", mode);
        println!("Service will start automatically on boot");
        println!("\nManual control:");
        println!("  Start:   systemctl {}start {}", user_flag, service_name);
        println!("  Stop:    systemctl {}stop {}", user_flag, service_name);
        println!("  Status:  systemctl {}status {}", user_flag, service_name);
        println!("  Logs:    journalctl {}-u {}", user_flag, service_name);

        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        error!("System service installation not supported on this platform");
        Err("Unsupported platform".into())
    }
}

/// Uninstall system service.
async fn daemon_uninstall(label: &str, _system: bool) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        use autohands_daemon::launchd::{LaunchAgent, LaunchAgentConfig};

        let config = LaunchAgentConfig::with_label(label);
        let agent = LaunchAgent::new(config);

        if !agent.is_installed() {
            info!("LaunchAgent not installed");
            return Ok(());
        }

        agent.uninstall()?;
        info!("Successfully uninstalled LaunchAgent: {}", label);
        println!("\nAutoHands LaunchAgent uninstalled");

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use autohands_daemon::systemd::{SystemdService, SystemdConfig};

        let service_name = label.replace("com.", "").replace('.', "-");

        let mut config = SystemdConfig::with_name(&service_name);
        if _system {
            config = config.system_mode();
        } else {
            config = config.user_mode();
        }

        let service = SystemdService::new(config);

        if !service.is_installed() {
            info!("Systemd service not installed");
            return Ok(());
        }

        service.uninstall()?;
        info!("Successfully uninstalled Systemd service: {}", service_name);
        println!("\nAutoHands Systemd service uninstalled");

        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        error!("System service uninstallation not supported on this platform");
        Err("Unsupported platform".into())
    }
}

/// Show system service logs.
async fn daemon_logs(label: &str, lines: u32) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        // Read log files
        let log_dir = dirs::home_dir()
            .map(|h| h.join(".autohands").join("logs"))
            .unwrap_or_else(|| PathBuf::from("/tmp"));

        let stdout_path = log_dir.join("stdout.log");
        let stderr_path = log_dir.join("stderr.log");

        println!("=== AutoHands Logs (LaunchAgent: {}) ===\n", label);

        // Read stdout log
        if stdout_path.exists() {
            println!("--- stdout.log ---");
            let content = std::fs::read_to_string(&stdout_path)?;
            let log_lines: Vec<&str> = content.lines().collect();
            let start = log_lines.len().saturating_sub(lines as usize);
            for line in &log_lines[start..] {
                println!("{}", line);
            }
        } else {
            println!("No stdout.log found at {}", stdout_path.display());
        }

        println!();

        // Read stderr log
        if stderr_path.exists() {
            println!("--- stderr.log ---");
            let content = std::fs::read_to_string(&stderr_path)?;
            let log_lines: Vec<&str> = content.lines().collect();
            let start = log_lines.len().saturating_sub(lines as usize);
            for line in &log_lines[start..] {
                println!("{}", line);
            }
        } else {
            println!("No stderr.log found at {}", stderr_path.display());
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use autohands_daemon::systemd::{SystemdService, SystemdConfig};

        let service_name = label.replace("com.", "").replace('.', "-");
        let config = SystemdConfig::with_name(&service_name).user_mode();
        let service = SystemdService::new(config);

        println!("=== AutoHands Logs (Systemd: {}) ===\n", service_name);

        let logs = service.logs(lines)?;
        println!("{}", logs);

        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        error!("Log viewing not supported on this platform");
        Err("Unsupported platform".into())
    }
}
