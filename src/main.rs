//! AutoHands - Omnipotent Autonomous Agent Framework
//!
//! Main entry point for the AutoHands CLI and server.

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use tracing::{error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use autohands_core::registry::{ChannelRegistry, ProviderRegistry, ToolRegistry};
use autohands_core::Kernel;

// Channel extensions
use autohands_channel_web::{WebChannel, WebChannelConfig};
use autohands_daemon::{Daemon, DaemonConfig, DaemonError};
use autohands_api::{AppState, InterfaceConfig, InterfaceServer};
use autohands_provider_anthropic::AnthropicProvider;
use autohands_runtime::{AgentLoopConfig, AgentRuntime, AgentRuntimeConfig};

// Tool extensions
use autohands_tools_filesystem::FilesystemExtension;
use autohands_tools_shell::ShellExtension;
use autohands_tools_browser::BrowserToolsExtension;
use autohands_tools_desktop::DesktopToolsExtension;
use autohands_tools_search::SearchExtension;
use autohands_tools_web::WebToolsExtension;
use autohands_tools_code::{AnalyzeCodeTool, FindSymbolTool};
use autohands_tools_skill::SkillToolsExtension;

// Agent extension
use autohands_agent_general::GeneralAgent;

// Skills - DynamicSkillLoader imported below with SkillPackager, SkillSource

// Protocols for extension context
use autohands_protocols::extension::Extension;
use autohands_protocols::agent::AgentConfig;

// Skills progressive disclosure
use autohands_skills_dynamic::SkillMetadataInjector;

/// AutoHands CLI.
#[derive(Parser)]
#[command(name = "autohands")]
#[command(about = "Omnipotent Autonomous Agent Framework")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config/default.toml", global = true)]
    config: PathBuf,

    /// Working directory
    #[arg(short, long, global = true)]
    work_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
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
enum SkillAction {
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
enum DaemonAction {
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

fn default_pid_file() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".autohands").join("autohands.pid"))
        .unwrap_or_else(|| PathBuf::from("/tmp/autohands.pid"))
}

/// Get the .autohands directory path.
fn autohands_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".autohands"))
        .unwrap_or_else(|| PathBuf::from(".autohands"))
}

/// Initialize tracing with console and file output.
///
/// Log files are written to ~/.autohands/debug/ with daily rotation and 100MB max size.
fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    // Create log directory
    let log_dir = autohands_dir().join("debug");
    std::fs::create_dir_all(&log_dir)?;

    // Create rolling file appender (daily rotation, max 100MB implied by daily rotation)
    // tracing-appender doesn't support size-based rotation natively, but daily rotation
    // combined with file naming helps manage log files
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("autohands")
        .filename_suffix("log")
        .max_log_files(30) // Keep 30 days of logs
        .build(&log_dir)?;

    // Create a non-blocking writer for file output
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Store the guard in a static to keep it alive for the program duration
    // This is a common pattern for tracing-appender
    static GUARD: std::sync::OnceLock<tracing_appender::non_blocking::WorkerGuard> =
        std::sync::OnceLock::new();
    let _ = GUARD.set(_guard);

    // Build subscriber with both console and file layers
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        // Console layer (human-readable text format with colors)
        .with(
            fmt::layer()
                .with_target(true)
                .with_ansi(true)
        )
        // File layer (text format without colors)
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
        )
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with file and console output
    init_tracing()?;

    let cli = Cli::parse();

    let work_dir = cli
        .work_dir
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    match cli.command {
        None => {
            // Default: run server with default ports
            run_server(work_dir, "127.0.0.1".to_string(), 8080, 8081).await
        }
        Some(Commands::Run { host, port, web_port }) => {
            run_server(work_dir, host, port, web_port).await
        }
        Some(Commands::Daemon { action }) => {
            handle_daemon_command(action, work_dir).await
        }
        Some(Commands::Skill { action }) => {
            handle_skill_command(action).await
        }
    }
}

/// Run the server in foreground.
async fn run_server(
    work_dir: PathBuf,
    host: String,
    port: u16,
    web_port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting AutoHands v{}", env!("CARGO_PKG_VERSION"));
    info!("Working directory: {}", work_dir.display());

    // Initialize kernel
    let kernel = Arc::new(Kernel::new(work_dir.clone()));
    info!("Kernel initialized");

    // Initialize registries
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let channel_registry = Arc::new(ChannelRegistry::new());

    // Register providers based on available API keys
    register_providers(&provider_registry).await;

    // Register tools and get skill registry for agent system prompt injection
    let skill_registry = register_tools_with_skill_registry(
        tool_registry.clone(),
        provider_registry.clone(),
        &work_dir,
    ).await;

    // Create AgentRuntime
    let runtime_config = AgentRuntimeConfig {
        max_concurrent: 10,
        default_loop_config: AgentLoopConfig {
            max_turns: 50,
            timeout_seconds: 300,
            checkpoint_enabled: false,
        },
    };
    let agent_runtime = Arc::new(AgentRuntime::new(
        provider_registry.clone(),
        tool_registry.clone(),
        runtime_config,
    ));

    // Register agents with skill metadata injected into system prompt
    register_agents(
        &agent_runtime,
        provider_registry.clone(),
        tool_registry.clone(),
        skill_registry,
    ).await;

    // Configure transcript directory for session recording
    // Use ~/.autohands/sessions for agent session transcripts
    let transcript_dir = autohands_dir().join("sessions");
    std::fs::create_dir_all(&transcript_dir).expect("Failed to create sessions directory");
    info!("Session transcripts will be saved to: {}", transcript_dir.display());

    // Create app state
    let state = Arc::new(AppState::new(
        provider_registry.clone(),
        tool_registry.clone(),
        kernel.clone(),
        agent_runtime.clone(),
        transcript_dir,
    ));

    // Create and start RunLoop
    use autohands_runloop::{ChannelBridge, RunLoop, RunLoopConfig, RunLoopMode};
    use autohands_api::RunLoopState;
    use std::time::Duration;

    let runloop_config = RunLoopConfig::default();
    let run_loop = Arc::new(RunLoop::new(runloop_config));

    // Create RunLoop state for HTTP API
    let runloop_state = Arc::new(RunLoopState::from_runloop(run_loop.clone()));

    // Initialize Web Channel
    let web_channel_config = WebChannelConfig {
        host: host.clone(),
        port: web_port,
    };
    let web_channel = Arc::new(WebChannel::new("web", web_channel_config));
    channel_registry
        .register(web_channel.clone())
        .expect("Failed to register web channel");

    // Start all channels
    channel_registry
        .start_all()
        .await
        .expect("Failed to start channels");
    info!("Web Channel started at http://{}:{}", host, web_port);

    // Create and start channel bridge (connects channels to RunLoop)
    let channel_bridge = ChannelBridge::new(
        channel_registry.clone(),
        run_loop.clone(),
    );
    channel_bridge.start().await;
    info!("ChannelBridge started, listening on {} channel(s)", channel_registry.list_ids().len());

    // Configure RunLoop with handler and channel registry
    use autohands_runloop::RuntimeAgentEventHandler;
    let handler = Arc::new(RuntimeAgentEventHandler::new(agent_runtime.clone(), "general"));
    run_loop.set_handler(handler).await;
    run_loop.set_channel_registry(channel_registry.clone()).await;
    info!("RunLoop configured with agent handler and channel registry");

    // Start RunLoop in background (run for 100 years = effectively forever)
    let run_loop_handle = run_loop.clone();
    tokio::spawn(async move {
        info!("Starting RunLoop...");
        // 100 years in seconds (effectively infinite for our purposes)
        let forever = Duration::from_secs(100 * 365 * 24 * 60 * 60);
        match run_loop_handle.run_in_mode(RunLoopMode::Default, forever).await {
            Ok(result) => info!("RunLoop finished: {:?}", result),
            Err(e) => error!("RunLoop error: {}", e),
        }
    });

    let config = InterfaceConfig::new(&host, port);
    let server = InterfaceServer::new(config, state, runloop_state);

    info!("AutoHands ready:");
    info!("  API Server:    http://{}:{}", host, port);
    info!("  Web Channel:   http://{}:{}", host, web_port);
    info!("");
    info!("API Endpoints:");
    info!("  POST /tasks          - 提交任务");
    info!("  GET  /tasks/{{id}}     - 查询状态");
    info!("  POST /webhook/{{id}}   - 触发 Webhook");
    info!("  GET  /ws             - WebSocket");

    // Run server (this will block until shutdown)
    server.run().await?;

    info!("Shutting down...");
    Ok(())
}

/// Register available tools and return skill registry for agent system prompt injection.
async fn register_tools_with_skill_registry(
    tool_registry: Arc<ToolRegistry>,
    provider_registry: Arc<ProviderRegistry>,
    work_dir: &PathBuf,
) -> Arc<autohands_skills_dynamic::SkillRegistry> {
    use autohands_core::registry::MemoryRegistry;
    use autohands_protocols::extension::ExtensionContext;

    // Create extension context for initializing extensions
    // Note: task_submitter is None since we're not running within a RunLoop here
    let memory_registry = Arc::new(MemoryRegistry::new());

    let ctx = ExtensionContext::new(
        serde_json::Value::Null,
        None, // task_submitter - not needed for tool registration
        tool_registry.clone() as Arc<dyn autohands_protocols::extension::ToolRegistryAccess>,
        provider_registry.clone() as Arc<dyn autohands_protocols::extension::ProviderRegistryAccess>,
        memory_registry as Arc<dyn autohands_protocols::extension::MemoryRegistryAccess>,
        work_dir.clone(),
    );

    // Register Filesystem tools
    let mut fs_ext = FilesystemExtension::new();
    match fs_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = fs_ext.manifest().provides.tools.clone();
            info!("Registered filesystem tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize filesystem extension: {}", e);
        }
    }

    // Register Shell tools
    let mut shell_ext = ShellExtension::new();
    match shell_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = shell_ext.manifest().provides.tools.clone();
            info!("Registered shell tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize shell extension: {}", e);
        }
    }

    // Register Browser tools - Chrome will be auto-launched on first use
    // Profile persisted at ~/.autohands/browser-profile for login state
    let mut browser_ext = BrowserToolsExtension::new();
    match browser_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = browser_ext.manifest().provides.tools.clone();
            info!("Registered browser tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize browser extension: {}", e);
        }
    }

    // Register Desktop tools
    let mut desktop_ext = DesktopToolsExtension::new();
    match desktop_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = desktop_ext.manifest().provides.tools.clone();
            info!("Registered desktop tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize desktop extension: {}", e);
        }
    }

    // Register Search tools (glob, grep)
    let mut search_ext = SearchExtension::new();
    match search_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = search_ext.manifest().provides.tools.clone();
            info!("Registered search tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize search extension: {}", e);
        }
    }

    // Register Web tools (web_fetch, web_search)
    let mut web_ext = WebToolsExtension::new();
    match web_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = web_ext.manifest().provides.tools.clone();
            info!("Registered web tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize web extension: {}", e);
        }
    }

    // Register Code tools (analyze_code, find_symbol) - no Extension, register directly
    if let Err(e) = tool_registry.register(Arc::new(AnalyzeCodeTool::new())) {
        warn!("Failed to register analyze_code tool: {}", e);
    } else {
        info!("Registered analyze_code tool");
    }
    if let Err(e) = tool_registry.register(Arc::new(FindSymbolTool::new())) {
        warn!("Failed to register find_symbol tool: {}", e);
    } else {
        info!("Registered find_symbol tool");
    }

    // Create skill registry and loader
    let skill_registry = Arc::new(autohands_skills_dynamic::SkillRegistry::new());
    let skill_loader = create_skill_loader_for_server(work_dir).await;

    // Load skills into registry
    {
        use autohands_protocols::skill::SkillLoader;
        if let Ok(skill_defs) = skill_loader.list().await {
            for def in &skill_defs {
                if let Ok(skill) = skill_loader.load(&def.id).await {
                    skill_registry.register(skill).await;
                }
            }
            info!("Loaded {} skills into registry for progressive disclosure", skill_defs.len());
        }
    }

    // Register Skill tools with the loader
    let skill_loader: Arc<tokio::sync::RwLock<dyn autohands_protocols::skill::SkillLoader>> =
        Arc::new(tokio::sync::RwLock::new(skill_loader));

    let mut skill_ext = SkillToolsExtension::new(skill_loader);
    match skill_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = skill_ext.manifest().provides.tools.clone();
            info!("Registered skill tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize skill tools extension: {}", e);
        }
    }

    // Log total registered tools
    let total_tools = tool_registry.list().len();
    info!("Total registered tools: {}", total_tools);

    skill_registry
}

/// Register available agents with skill metadata injected into system prompt.
async fn register_agents(
    agent_runtime: &AgentRuntime,
    provider_registry: Arc<ProviderRegistry>,
    tool_registry: Arc<ToolRegistry>,
    skill_registry: Arc<autohands_skills_dynamic::SkillRegistry>,
) {
    // Get first available provider for the default agent
    let provider_ids = provider_registry.list_ids();
    if provider_ids.is_empty() {
        warn!("No providers available, cannot create agents");
        return;
    }

    let default_provider_id = &provider_ids[0];
    let provider = provider_registry.get(default_provider_id).unwrap();

    // Use doubao-seed-1-8-251228 as the default model
    // Note: For Ark platform, you may need to use your endpoint ID instead
    let default_model = "doubao-seed-1-8-251228".to_string();

    // Collect all registered tools
    let tool_defs = tool_registry.list();
    let tools: Vec<Arc<dyn autohands_protocols::tool::Tool>> = tool_defs
        .iter()
        .filter_map(|def| tool_registry.get(&def.id))
        .collect();

    // Generate skill metadata section for system prompt (Progressive Disclosure L1)
    let skill_injector = SkillMetadataInjector::new(skill_registry.clone());
    let skill_section = skill_injector.generate_system_prompt_section().await;

    // Create general agent config with skill metadata in system prompt
    let mut agent_config = AgentConfig::new("general", "General Agent", &default_model);

    // Build system prompt with skill metadata
    let base_prompt = r#"You are AutoHands, an omnipotent autonomous agent capable of executing any task.

You have access to various tools for:
- File operations (read, write, edit, glob, grep)
- Shell commands
- Browser automation
- Desktop control (mouse, keyboard, screenshots, OCR)
- Web fetching and searching
- Code analysis

Execute tasks efficiently and thoroughly."#;

    agent_config.system_prompt = if skill_section.is_empty() {
        Some(base_prompt.to_string())
    } else {
        Some(format!("{}\n{}", base_prompt, skill_section))
    };

    // Log skill injection status
    let skill_count = skill_registry.len().await;
    if skill_count > 0 {
        info!(
            "Injected {} skill(s) metadata into agent system prompt (Progressive Disclosure L1)",
            skill_count
        );
    }

    // Create and register general agent
    let general_agent = GeneralAgent::new(agent_config, provider.clone(), tools);
    agent_runtime.register_agent(Arc::new(general_agent));

    info!("Registered general agent with model: {}", default_model);
    info!("Total registered agents: {}", agent_runtime.list_agents().len());
}

/// Register available LLM providers based on environment variables.
async fn register_providers(registry: &ProviderRegistry) {
    // Register Anthropic provider if API key is available
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        let provider = AnthropicProvider::new(api_key);
        if let Err(e) = registry.register(Arc::new(provider)) {
            warn!("Failed to register Anthropic provider: {}", e);
        } else {
            info!("Registered Anthropic provider");
        }
    }

    // Register Ark provider if API key is available
    if let Ok(api_key) = std::env::var("ARK_API_KEY") {
        let provider = autohands_provider_ark::ArkProvider::new(api_key);
        if let Err(e) = registry.register(Arc::new(provider)) {
            warn!("Failed to register Ark provider: {}", e);
        } else {
            info!("Registered Ark provider (豆包)");
        }
    }

    let provider_ids = registry.list_ids();
    if provider_ids.is_empty() {
        warn!("No LLM providers registered. Set ANTHROPIC_API_KEY or ARK_API_KEY environment variable.");
    } else {
        info!("Registered providers: {:?}", provider_ids);
    }
}

/// Handle daemon subcommands.
async fn handle_daemon_command(
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

// ============================================================================
// Skill Commands
// ============================================================================

use autohands_skills_dynamic::{DynamicSkillLoader, SkillPackager, SkillSource};

/// Handle skill subcommands.
async fn handle_skill_command(action: SkillAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        SkillAction::List { tag, category, format } => {
            skill_list(tag, category, &format).await
        }
        SkillAction::Info { skill_id } => {
            skill_info(&skill_id).await
        }
        SkillAction::Reload => {
            skill_reload().await
        }
        SkillAction::Pack { skill_dir, output } => {
            skill_pack(&skill_dir, output.as_deref()).await
        }
        SkillAction::Install { skill_file, dir } => {
            skill_install(&skill_file, dir.as_deref()).await
        }
        SkillAction::New { skill_id, name, output } => {
            skill_new(&skill_id, name.as_deref(), output.as_deref()).await
        }
    }
}

/// List all available skills.
async fn skill_list(
    tag: Option<String>,
    category: Option<String>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let loader = create_skill_loader().await;
    loader.load_all().await?;

    use autohands_protocols::skill::SkillLoader;
    let skills = loader.list().await?;

    // Filter by tag or category
    let filtered: Vec<_> = skills
        .into_iter()
        .filter(|s| {
            if let Some(ref t) = tag {
                if !s.tags.contains(t) {
                    return false;
                }
            }
            if let Some(ref c) = category {
                if s.category.as_ref() != Some(c) {
                    return false;
                }
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        println!("No skills found.");
        return Ok(());
    }

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&filtered)?;
            println!("{}", json);
        }
        _ => {
            // Table format
            println!("{:<20} {:<30} {:<15} {}", "ID", "NAME", "CATEGORY", "TAGS");
            println!("{}", "-".repeat(80));
            for skill in filtered {
                let category = skill.category.as_deref().unwrap_or("-");
                let tags = skill.tags.join(", ");
                println!("{:<20} {:<30} {:<15} {}", skill.id, skill.name, category, tags);
            }
        }
    }

    Ok(())
}

/// Show detailed info about a skill.
async fn skill_info(skill_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let loader = create_skill_loader().await;
    loader.load_all().await?;

    use autohands_protocols::skill::SkillLoader;
    let skill = loader.load(skill_id).await?;

    println!("Skill: {}", skill.definition.name);
    println!("{}", "=".repeat(50));
    println!("ID:          {}", skill.definition.id);
    println!("Description: {}", skill.definition.description);
    if let Some(cat) = &skill.definition.category {
        println!("Category:    {}", cat);
    }
    if !skill.definition.tags.is_empty() {
        println!("Tags:        {}", skill.definition.tags.join(", "));
    }
    println!("Priority:    {}", skill.definition.priority);
    println!("Enabled:     {}", skill.definition.enabled);

    if !skill.definition.required_tools.is_empty() {
        println!("Required Tools: {}", skill.definition.required_tools.join(", "));
    }

    if !skill.definition.variables.is_empty() {
        println!("\nVariables:");
        for var in &skill.definition.variables {
            let required = if var.required { " (required)" } else { "" };
            let default = var.default.as_ref().map(|d| format!(" [default: {}]", d)).unwrap_or_default();
            println!("  - {}: {}{}{}", var.name, var.description, required, default);
        }
    }

    println!("\nContent Preview:");
    println!("{}", "-".repeat(50));
    // Show first 500 chars
    let preview: String = skill.content.chars().take(500).collect();
    println!("{}", preview);
    if skill.content.len() > 500 {
        println!("... ({} more characters)", skill.content.len() - 500);
    }

    Ok(())
}

/// Reload all skills.
async fn skill_reload() -> Result<(), Box<dyn std::error::Error>> {
    let loader = create_skill_loader().await;

    use autohands_protocols::skill::SkillLoader;
    loader.reload().await?;

    let skills = loader.list().await?;
    println!("Reloaded {} skills", skills.len());

    Ok(())
}

/// Pack a skill directory.
async fn skill_pack(
    skill_dir: &PathBuf,
    output: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = output
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let package_path = SkillPackager::pack(skill_dir, &output_dir)?;
    println!("Created skill package: {}", package_path.display());

    Ok(())
}

/// Install a skill package.
async fn skill_install(
    skill_file: &PathBuf,
    dir: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let skills_dir = dir
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".autohands").join("skills"))
                .unwrap_or_else(|| PathBuf::from("./skills"))
        });

    // Ensure directory exists
    std::fs::create_dir_all(&skills_dir)?;

    let installed_path = SkillPackager::install(skill_file, &skills_dir)?;
    println!("Installed skill to: {}", installed_path.display());

    Ok(())
}

/// Create a new skill from template.
async fn skill_new(
    skill_id: &str,
    name: Option<&str>,
    output: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let skills_dir = output
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".autohands").join("skills"))
                .unwrap_or_else(|| PathBuf::from("./skills"))
        });

    // Ensure directory exists
    std::fs::create_dir_all(&skills_dir)?;

    let skill_name = name.unwrap_or(skill_id);
    let skill_path = skills_dir.join(format!("{}.markdown", skill_id));

    if skill_path.exists() {
        return Err(format!("Skill already exists: {}", skill_path.display()).into());
    }

    let template = format!(
        r#"---
id: {}
name: {}
version: 1.0.0
description: Description of your skill

requires:
  tools: []
  bins: []

tags: []
category: general
priority: 10

variables: []
---

# {}

Your skill prompt content here.

## Instructions

1. Step one
2. Step two
3. Step three

## Guidelines

- Guideline one
- Guideline two
"#,
        skill_id, skill_name, skill_name
    );

    std::fs::write(&skill_path, template)?;
    println!("Created new skill: {}", skill_path.display());
    println!("\nEdit the file to customize your skill.");

    Ok(())
}

/// Create a skill loader with default configuration (for CLI commands).
async fn create_skill_loader() -> DynamicSkillLoader {
    let mut loader = DynamicSkillLoader::new();

    // Add workspace directory if exists
    if let Ok(cwd) = std::env::current_dir() {
        let workspace = cwd.join("skills");
        if workspace.exists() {
            loader = loader.with_source(SkillSource::Workspace(workspace));
        }
    }

    loader
}

/// Create a skill loader for the server with all skills loaded.
async fn create_skill_loader_for_server(work_dir: &PathBuf) -> DynamicSkillLoader {
    let mut loader = DynamicSkillLoader::new();

    // Add workspace directory if exists
    let workspace = work_dir.join("skills");
    if workspace.exists() {
        loader = loader.with_source(SkillSource::Workspace(workspace));
    }

    // Load all skills
    if let Err(e) = loader.load_all().await {
        warn!("Failed to load skills: {}", e);
    } else {
        use autohands_protocols::skill::SkillLoader;
        match loader.list().await {
            Ok(skills) => {
                info!("Loaded {} skills for Agent use", skills.len());
                for skill in &skills {
                    info!("  - {}: {}", skill.id, skill.name);
                }
            }
            Err(e) => warn!("Failed to list skills: {}", e),
        }
    }

    loader
}
