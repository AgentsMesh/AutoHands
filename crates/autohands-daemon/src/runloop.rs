//! RunLoop integration for Daemon.
//!
//! Provides a RunLoop-driven main function that can be passed to Daemon::run().
//! This integrates the event-driven RunLoop architecture with the Daemon lifecycle.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tracing::{debug, error, info};

use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_runloop::{
    CheckpointObserver, TaskPriority,
    HealthCheckObserver, LivenessCheck, MemoryCheck, MemoryCheckpointManager, MetricsObserver,
    RunLoop, RunLoopConfig, RunLoopMode, SignalEvent, SignalSource1, Timer,
    TimerBuilder,
};
use autohands_runtime::{AgentLoopConfig, AgentRuntime, AgentRuntimeConfig};

use crate::error::DaemonError;

/// RunLoop-based daemon runner.
///
/// This struct encapsulates the RunLoop configuration and provides
/// a run method suitable for passing to Daemon::run().
pub struct RunLoopRunner {
    /// RunLoop configuration.
    config: RunLoopConfig,

    /// Provider registry.
    provider_registry: Arc<ProviderRegistry>,

    /// Tool registry.
    tool_registry: Arc<ToolRegistry>,

    /// Default agent ID.
    default_agent: String,

    /// Shutdown receiver from daemon.
    shutdown_rx: Option<broadcast::Receiver<()>>,
}

impl RunLoopRunner {
    /// Create a new RunLoopRunner.
    pub fn new(
        provider_registry: Arc<ProviderRegistry>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            config: RunLoopConfig::default(),
            provider_registry,
            tool_registry,
            default_agent: "general".to_string(),
            shutdown_rx: None,
        }
    }

    /// Set custom RunLoop configuration.
    pub fn with_config(mut self, config: RunLoopConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the default agent ID.
    pub fn with_default_agent(mut self, agent: impl Into<String>) -> Self {
        self.default_agent = agent.into();
        self
    }

    /// Set the shutdown receiver from daemon.
    pub fn with_shutdown_receiver(mut self, rx: broadcast::Receiver<()>) -> Self {
        self.shutdown_rx = Some(rx);
        self
    }

    /// Run the RunLoop-driven daemon.
    ///
    /// This method creates and runs the RunLoop with all necessary components:
    /// - AgentRuntime for agent execution
    /// - RuntimeAgentEventHandler bridging RunLoop tasks to AgentRuntime
    /// - Observers for health checks, metrics, and checkpoints
    /// - Timer for periodic health checks
    /// - Signal source for daemon signal bridging
    pub async fn run(self) -> Result<(), DaemonError> {
        info!("Initializing RunLoop-driven daemon...");

        // Create RunLoop
        let run_loop = Arc::new(RunLoop::new(self.config.clone()));
        info!("RunLoop created");

        // Create AgentRuntime
        let runtime_config = AgentRuntimeConfig {
            max_concurrent: self.config.workers.max_workers,
            default_loop_config: AgentLoopConfig {
                checkpoint_enabled: false,
                ..Default::default()
            },
        };
        let agent_runtime = Arc::new(AgentRuntime::new(
            self.provider_registry.clone(),
            self.tool_registry.clone(),
            runtime_config,
        ));
        info!("AgentRuntime created");

        // Configure RunLoop with handler
        let handler: Arc<dyn autohands_runloop::AgentEventHandler> = Arc::new(
            autohands_runloop::RuntimeAgentEventHandler::new(
                agent_runtime.clone(),
                self.default_agent.clone(),
            ),
        );
        run_loop.set_handler(handler).await;
        info!("RunLoop: Agent event handler configured for daemon mode");

        // Register observers
        self.register_observers(&run_loop).await;

        // Create heartbeat timer
        let heartbeat_timer = self.create_heartbeat_timer(&run_loop);
        info!("Heartbeat timer created (interval: 30s)");

        // Set up signal bridging
        let signal_source = SignalSource1::new();
        let (signal_receiver, signal_sender) = signal_source.create_receiver();
        run_loop.add_source1(signal_receiver).await;
        info!("Signal source registered");

        // Bridge daemon shutdown signal to RunLoop
        if let Some(mut shutdown_rx) = self.shutdown_rx {
            let run_loop_clone = run_loop.clone();
            let signal_sender_clone = signal_sender.clone();
            tokio::spawn(async move {
                if shutdown_rx.recv().await.is_ok() {
                    info!("Daemon shutdown signal received, stopping RunLoop...");
                    let _ = signal_sender_clone.send(SignalEvent::Shutdown).await;
                    run_loop_clone.stop();
                }
            });
        }

        // Run the main loop
        info!("RunLoop starting in Default mode...");
        let result = run_loop
            .run_in_mode(RunLoopMode::Default, Duration::MAX)
            .await;

        // Cleanup
        heartbeat_timer.cancel();

        match result {
            Ok(run_result) => {
                info!("RunLoop exited: {:?}", run_result);
                Ok(())
            }
            Err(e) => {
                error!("RunLoop error: {}", e);
                Err(DaemonError::Custom(format!("RunLoop error: {}", e)))
            }
        }
    }

    /// Register standard observers.
    async fn register_observers(&self, run_loop: &RunLoop) {
        // Metrics observer
        let metrics_observer = Arc::new(MetricsObserver::new());
        run_loop.add_observer("metrics", metrics_observer).await;
        debug!("Registered MetricsObserver");

        // Health check observer
        let health_observer = Arc::new(HealthCheckObserver::new(3)); // 3 consecutive failures threshold
        health_observer.register(Arc::new(LivenessCheck));
        health_observer.register(Arc::new(MemoryCheck::new(90)));
        run_loop.add_observer("health", health_observer).await;
        debug!("Registered HealthCheckObserver");

        // Checkpoint observer
        let checkpoint_manager = Arc::new(MemoryCheckpointManager::new(10));
        let checkpoint_observer = Arc::new(CheckpointObserver::new(checkpoint_manager));
        run_loop
            .add_observer("checkpoint", checkpoint_observer)
            .await;
        debug!("Registered CheckpointObserver");

        info!("All observers registered");
    }

    /// Create heartbeat timer.
    fn create_heartbeat_timer(&self, run_loop: &Arc<RunLoop>) -> Arc<Timer> {
        TimerBuilder::new()
            .id("daemon-heartbeat")
            .interval(Duration::from_secs(30))
            .repeating()
            .task_type("system:heartbeat")
            .priority(TaskPriority::Low)
            .build(run_loop.clone())
    }
}

/// Builder for creating a RunLoop-driven daemon.
pub struct RunLoopDaemonBuilder {
    config: RunLoopConfig,
    provider_registry: Option<Arc<ProviderRegistry>>,
    tool_registry: Option<Arc<ToolRegistry>>,
    default_agent: String,
    shutdown_rx: Option<broadcast::Receiver<()>>,
}

impl RunLoopDaemonBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: RunLoopConfig::default(),
            provider_registry: None,
            tool_registry: None,
            default_agent: "general".to_string(),
            shutdown_rx: None,
        }
    }

    /// Set RunLoop configuration.
    pub fn config(mut self, config: RunLoopConfig) -> Self {
        self.config = config;
        self
    }

    /// Set provider registry.
    pub fn provider_registry(mut self, registry: Arc<ProviderRegistry>) -> Self {
        self.provider_registry = Some(registry);
        self
    }

    /// Set tool registry.
    pub fn tool_registry(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// Set default agent.
    pub fn default_agent(mut self, agent: impl Into<String>) -> Self {
        self.default_agent = agent.into();
        self
    }

    /// Set shutdown receiver.
    pub fn shutdown_receiver(mut self, rx: broadcast::Receiver<()>) -> Self {
        self.shutdown_rx = Some(rx);
        self
    }

    /// Build the RunLoopRunner.
    pub fn build(self) -> Result<RunLoopRunner, &'static str> {
        let provider_registry = self
            .provider_registry
            .ok_or("provider_registry is required")?;
        let tool_registry = self.tool_registry.ok_or("tool_registry is required")?;

        let mut runner = RunLoopRunner::new(provider_registry, tool_registry)
            .with_config(self.config)
            .with_default_agent(self.default_agent);

        if let Some(rx) = self.shutdown_rx {
            runner = runner.with_shutdown_receiver(rx);
        }

        Ok(runner)
    }
}

impl Default for RunLoopDaemonBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "runloop_tests.rs"]
mod tests;
