//! AutoHands - Omnipotent Autonomous Agent Framework
//!
//! Main entry point for the AutoHands CLI and server.

mod adapters;
mod cli;
mod cmd_daemon;
mod cmd_skill;
mod register;
mod server;

use clap::Parser;
use tracing::{info, warn};

use autohands_config::{ConfigLoader, Config};
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with file and console output
    server::init_tracing()?;

    let cli = Cli::parse();

    // Load configuration from file (with env var expansion fallback)
    let config = ConfigLoader::load(&cli.config).unwrap_or_else(|e| {
        warn!("Failed to load config from {:?}: {}, using defaults", cli.config, e);
        Config::default()
    });
    info!("Configuration loaded: server={}:{}", config.server.host, config.server.port);

    let work_dir = cli
        .work_dir
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    match cli.command {
        None => {
            // Default: run server with config
            server::run_server(work_dir, config).await
        }
        Some(Commands::Run { host, port, web_port: _ }) => {
            // CLI args override config values
            let mut config = config;
            config.server.host = host;
            config.server.port = port;
            server::run_server(work_dir, config).await
        }
        Some(Commands::Daemon { action }) => {
            cmd_daemon::handle_daemon_command(action, work_dir).await
        }
        Some(Commands::Skill { action }) => {
            cmd_skill::handle_skill_command(action).await
        }
    }
}
