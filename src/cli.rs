//! CLI definitions for AutoHands.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// AutoHands CLI.
#[derive(Parser)]
#[command(name = "autohands")]
#[command(about = "Omnipotent Autonomous Agent Framework")]
#[command(version)]
pub(crate) struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config/default.toml", global = true)]
    pub config: PathBuf,

    /// Working directory
    #[arg(short, long, global = true)]
    pub work_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Run the server in foreground (default)
    Run {
        /// Server host
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Server port (API)
        #[arg(long, default_value_t = 8080)]
        port: u16,

        /// Web channel port (WebSocket UI)
        #[arg(long, default_value_t = 8081)]
        web_port: u16,
    },

    /// Daemon management commands
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Skill management commands
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
}

#[derive(Subcommand)]
pub(crate) enum SkillAction {
    /// List all available skills
    List {
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,

        /// Filter by category
        #[arg(long)]
        category: Option<String>,

        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Show detailed info about a skill
    Info {
        /// Skill ID
        skill_id: String,
    },

    /// Reload all skills from disk
    Reload,

    /// Pack a skill directory into a .skill file
    Pack {
        /// Path to skill directory
        skill_dir: PathBuf,

        /// Output directory (default: current directory)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Install a .skill package
    Install {
        /// Path to .skill file
        skill_file: PathBuf,

        /// Installation directory (default: ~/.autohands/skills/)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Create a new skill from template
    New {
        /// Skill ID
        skill_id: String,

        /// Skill name
        #[arg(short, long)]
        name: Option<String>,

        /// Output directory (default: ~/.autohands/skills/)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub(crate) enum DaemonAction {
    /// Start the daemon process
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,

        /// PID file path
        #[arg(long)]
        pid_file: Option<PathBuf>,
    },

    /// Stop the daemon process
    Stop {
        /// PID file path
        #[arg(long)]
        pid_file: Option<PathBuf>,

        /// Force kill if graceful shutdown fails
        #[arg(long)]
        force: bool,
    },

    /// Restart the daemon process
    Restart {
        /// PID file path
        #[arg(long)]
        pid_file: Option<PathBuf>,
    },

    /// Get daemon status
    Status {
        /// PID file path
        #[arg(long)]
        pid_file: Option<PathBuf>,
    },

    /// Install as system service (macOS LaunchAgent or Linux Systemd)
    Install {
        /// Service label/name
        #[arg(long, default_value = "com.autohands.agent")]
        label: String,

        /// Install as system service (requires root on Linux)
        #[arg(long)]
        system: bool,
    },

    /// Uninstall system service
    Uninstall {
        /// Service label/name
        #[arg(long, default_value = "com.autohands.agent")]
        label: String,

        /// Uninstall system service (requires root on Linux)
        #[arg(long)]
        system: bool,
    },

    /// Show system service logs
    Logs {
        /// Number of lines to show
        #[arg(long, default_value_t = 100)]
        lines: u32,

        /// Service label/name
        #[arg(long, default_value = "com.autohands.agent")]
        label: String,
    },
}
