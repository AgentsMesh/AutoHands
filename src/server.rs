//! Server initialization and startup logic for AutoHands.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::{error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use autohands_api::{AppState, InterfaceConfig};
use autohands_protocols::Channel;
use autohands_channel_web::{WebChannel, WebChannelConfig};
use autohands_checkpoint::{CheckpointConfig as CpConfig, CheckpointManager, FileCheckpointStore};
use autohands_config::{Config, ConfigLoader};
use autohands_core::registry::{ChannelRegistry, ProviderRegistry, ToolRegistry};
use autohands_core::Kernel;
use autohands_monitor::metrics::MetricsRegistry;
use autohands_runtime::{AgentLoopConfig, AgentRuntime, AgentRuntimeConfig};

use crate::adapters::{autohands_dir, CheckpointAdapter, MetricsWrappedHandler};
use crate::register::{register_agents, register_providers, register_tools_with_skill_registry};

/// Initialize tracing with console and file output.
///
/// Log files are written to ~/.autohands/debug/ with daily rotation and 100MB max size.
pub(crate) fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
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

/// Run the server in foreground.
pub(crate) async fn run_server(
    work_dir: PathBuf,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let host = config.server.host.clone();
    let port = config.server.port;
    // web_port defaults to port + 1 (e.g., 8080 -> 8081)
    let web_port = port + 1;

    info!("Starting AutoHands v{}", env!("CARGO_PKG_VERSION"));
    info!("Working directory: {}", work_dir.display());

    // Initialize kernel
    let kernel = Arc::new(Kernel::new(work_dir.clone()));
    info!("Kernel initialized");

    // Initialize registries
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let channel_registry = Arc::new(ChannelRegistry::new());

    // Register providers based on config and available API keys
    register_providers(&provider_registry, &config).await;

    // Register tools and get skill registry + memory backend + agent tools extension
    let (skill_registry, memory_backend, agent_tools_ext) = register_tools_with_skill_registry(
        tool_registry.clone(),
        provider_registry.clone(),
        &work_dir,
        &config,
    ).await;

    // Initialize checkpoint system
    let checkpoint_manager = if config.checkpoint.enabled {
        let storage_path = config.checkpoint.storage_path
            .clone()
            .map(|p| {
                let expanded = ConfigLoader::expand_path(&p.to_string_lossy());
                PathBuf::from(expanded)
            })
            .unwrap_or_else(|| autohands_dir().join("checkpoints"));
        std::fs::create_dir_all(&storage_path)?;

        let store = Arc::new(FileCheckpointStore::new(&storage_path).await?);
        let cp_config = CpConfig {
            enabled: true,
            interval_turns: config.checkpoint.interval_turns,
            storage_path: storage_path.clone(),
            max_checkpoints: config.checkpoint.max_checkpoints,
            auto_recover: true,
        };
        let manager = Arc::new(CheckpointManager::new(cp_config, store));
        info!("Checkpoint system initialized (interval={} turns, path={})",
            config.checkpoint.interval_turns, storage_path.display());
        Some(manager)
    } else {
        info!("Checkpoint system disabled");
        None
    };
    // Create AgentRuntime with config-driven values and optional checkpoint support
    let runtime_config = AgentRuntimeConfig {
        max_concurrent: 10,
        default_loop_config: AgentLoopConfig {
            checkpoint_enabled: config.checkpoint.enabled,
            ..Default::default()
        },
    };
    let mut agent_runtime = AgentRuntime::new(
        provider_registry.clone(),
        tool_registry.clone(),
        runtime_config,
    );

    if let Some(ref cp_manager) = checkpoint_manager {
        let adapter = Arc::new(CheckpointAdapter { manager: cp_manager.clone() });
        agent_runtime = agent_runtime.with_checkpoint(adapter);
        info!("Checkpoint support wired into AgentRuntime");
    }

    // Wire memory backend into AgentRuntime for context injection and flush
    if let Some(ref backend) = memory_backend {
        agent_runtime = agent_runtime.with_memory(backend.clone());
        info!("Memory backend wired into AgentRuntime");
    }

    // Create HistoryCompressor for context length recovery
    {
        use autohands_runtime::{HistoryCompressor, LLMSummarizer, SummarizerConfig};
        let provider_ids = provider_registry.list_ids();
        if !provider_ids.is_empty() {
            if let Some(provider) = provider_registry.get(&provider_ids[0]) {
                let summarizer_config = SummarizerConfig::default();
                let summarizer = Arc::new(LLMSummarizer::new(provider, summarizer_config.clone()));
                let compressor = Arc::new(HistoryCompressor::new(summarizer, summarizer_config));
                agent_runtime = agent_runtime.with_compressor(compressor);
                info!("HistoryCompressor wired into AgentRuntime");
            }
        }
    }

    let agent_runtime = Arc::new(agent_runtime);

    // Inject AgentRuntime into tools-agent extension (post-initialization)
    if let Some(ref ext) = agent_tools_ext {
        ext.set_runtime(agent_runtime.clone());
        info!("AgentRuntime injected into tools-agent extension");
    }

    // Register agents with skill metadata injected into system prompt
    register_agents(
        &agent_runtime,
        provider_registry.clone(),
        tool_registry.clone(),
        skill_registry,
    ).await;

    // Initialize monitor system
    let metrics_registry = Arc::new(MetricsRegistry::new());
    if config.monitor.enabled {
        metrics_registry.register_counter("autohands_requests_total", "Total requests").await;
        metrics_registry.register_counter("autohands_tasks_completed", "Tasks completed").await;
        metrics_registry.register_counter("autohands_tasks_failed", "Failed tasks").await;
        metrics_registry.register_gauge("autohands_active_sessions", "Active sessions").await;
        info!("Monitor system initialized (health={}, metrics={})",
            config.monitor.health_endpoint, config.monitor.metrics_endpoint);
    }

    // Configure transcript directory for session recording
    // Use ~/.autohands/sessions for agent session transcripts
    let transcript_dir = autohands_dir().join("sessions");
    std::fs::create_dir_all(&transcript_dir)?;
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
    channel_registry.register(web_channel.clone())?;

    // Start all channels
    channel_registry.start_all().await?;
    info!("Web Channel started at http://{}:{}", host, web_port);

    // Create and start channel bridge (connects channels to RunLoop)
    let channel_bridge = ChannelBridge::new(
        channel_registry.clone(),
        run_loop.clone(),
    );
    channel_bridge.start().await;
    info!("ChannelBridge started, listening on {} channel(s)", channel_registry.list_ids().len());

    // Configure RunLoop with handler (optionally wrapped with metrics) and channel registry
    use autohands_runloop::RuntimeAgentEventHandler;
    let inner_handler = Arc::new(RuntimeAgentEventHandler::new(agent_runtime.clone(), &config.agent.default));
    let handler: Arc<dyn autohands_runloop::AgentEventHandler> = if config.monitor.enabled {
        Arc::new(MetricsWrappedHandler {
            inner: inner_handler,
            metrics: metrics_registry.clone(),
            active_count: std::sync::atomic::AtomicU64::new(0),
        })
    } else {
        inner_handler
    };
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

    // Build the server with monitor routes merged in
    let interface_config = InterfaceConfig::new(&host, port);
    // Create API WebSocket Channel for response routing
    let api_ws_channel = Arc::new(autohands_api::ApiWsChannel::new());
    channel_registry.register(api_ws_channel.clone())?;
    api_ws_channel.start().await?;
    info!("API WebSocket Channel registered for response routing");

    let hybrid_state = Arc::new(autohands_api::HybridAppState::new(state.clone(), runloop_state, api_ws_channel));
    let base_router = autohands_api::create_router_with_hybrid_state(hybrid_state);

    // Monitor routes (/health, /metrics) are already built into the API router
    // via create_router_with_hybrid_state. No need to add them again here.
    let app = base_router;

    info!("AutoHands ready:");
    info!("  API Server:    http://{}:{}", host, port);
    info!("  Web Channel:   http://{}:{}", host, web_port);
    info!("");
    info!("API Endpoints:");
    info!("  POST /tasks          - 提交任务");
    info!("  GET  /tasks/{{id}}     - 查询状态");
    info!("  POST /webhook/{{id}}   - 触发 Webhook");
    info!("  GET  /ws             - WebSocket");
    if config.monitor.enabled {
        info!("  GET  {}       - 健康检查", config.monitor.health_endpoint);
        info!("  GET  {}      - Prometheus 指标", config.monitor.metrics_endpoint);
    }

    // Spawn periodic cleanup task for session, history, and transcript memory management (#6, #16)
    {
        let session_mgr = agent_runtime.session_manager().clone();
        let history_mgr = agent_runtime.history_manager().clone();
        let transcript_mgr = state.transcript_manager.clone();
        let agent_runtime_clone = agent_runtime.clone();
        tokio::spawn(async move {
            let cleanup_interval = std::time::Duration::from_secs(10 * 60); // 10 minutes
            let max_idle = std::time::Duration::from_secs(60 * 60); // 1 hour
            loop {
                tokio::time::sleep(cleanup_interval).await;
                // Skip sessions with running agents to avoid data loss
                let running_sessions = agent_runtime_clone.running_sessions();
                let expired = session_mgr.cleanup_with_exclusion(max_idle, &running_sessions);
                for session_id in &expired {
                    history_mgr.remove(session_id);
                    transcript_mgr.remove_writer(session_id).await;
                }
                if !expired.is_empty() {
                    info!(
                        "Periodic cleanup: removed {} idle session(s), remaining sessions={}, histories={}",
                        expired.len(),
                        session_mgr.count(),
                        history_mgr.session_count(),
                    );
                }
            }
        });
        info!("Periodic session/history cleanup task started (interval=10min, max_idle=1h)");
    }

    // Run server with graceful shutdown (#2)
    let addr: std::net::SocketAddr = format!("{}:{}", interface_config.host, interface_config.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Interface server listening on {}", addr);

    // Clone run_loop and shutdown_notify for use in the shutdown signal handler
    let shutdown_run_loop = run_loop.clone();
    let shutdown_notify = state.shutdown_notify.clone();
    let shutdown_signal = async move {
        let ctrl_c = tokio::signal::ctrl_c();
        let api_shutdown = shutdown_notify.notified();

        #[cfg(unix)]
        {
            let sigterm_result = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate());
            match sigterm_result {
                Ok(mut sigterm) => {
                    tokio::select! {
                        _ = ctrl_c => {
                            info!("Received Ctrl-C, initiating graceful shutdown...");
                        }
                        _ = sigterm.recv() => {
                            info!("Received SIGTERM, initiating graceful shutdown...");
                        }
                        _ = api_shutdown => {
                            info!("Received API shutdown request, initiating graceful shutdown...");
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to install SIGTERM handler: {}, falling back to Ctrl-C only", e);
                    tokio::select! {
                        _ = ctrl_c => {
                            info!("Received Ctrl-C, initiating graceful shutdown...");
                        }
                        _ = api_shutdown => {
                            info!("Received API shutdown request, initiating graceful shutdown...");
                        }
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            tokio::select! {
                _ = ctrl_c => {
                    info!("Received Ctrl-C, initiating graceful shutdown...");
                }
                _ = api_shutdown => {
                    info!("Received API shutdown request, initiating graceful shutdown...");
                }
            }
        }

        // Stop RunLoop so in-flight agents can flush checkpoints
        shutdown_run_loop.stop();

        // Grace period: allow in-flight tasks to complete
        info!("Waiting 5s grace period for in-flight tasks...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    info!("Shutting down...");
    Ok(())
}
