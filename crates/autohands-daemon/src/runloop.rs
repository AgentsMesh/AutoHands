//! RunLoop integration for Daemon.
//!
//! Provides a RunLoop-driven main function that can be passed to Daemon::run().
//! This integrates the event-driven RunLoop architecture with the Daemon lifecycle.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_runloop::{
    AgentDriver, AgentSource0, CheckpointObserver, TaskPriority, TaskSource,
    HealthCheckObserver, LivenessCheck, MemoryCheck, MemoryCheckpointManager, MetricsObserver,
    RunLoop, RunLoopConfig, Task, RunLoopMode, SignalEvent, SignalSource1, Timer,
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
    /// - AgentDriver for event-driven processing
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
                max_turns: 50,
                timeout_seconds: 300,
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

        // Create AgentSource0 for self-driving events
        let agent_source = Arc::new(AgentSource0::new("agent-events"));

        // Create AgentDriver with RuntimeAgentEventHandler
        let handler = Arc::new(autohands_runloop::RuntimeAgentEventHandler::new(
            agent_runtime.clone(),
            self.default_agent.clone(),
        ));
        let agent_driver = Arc::new(
            AgentDriver::new(run_loop.clone(), agent_source.clone(), self.config.clone())
                .with_handler(handler),
        );
        agent_driver.start();
        info!(
            "AgentDriver started with {} workers",
            self.config.workers.max_workers
        );

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
        agent_driver.stop();
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

/// RunLoop event handler that processes daemon-specific events.
pub struct DaemonEventHandler {
    /// AgentDriver for agent events.
    agent_driver: Arc<AgentDriver>,
}

impl DaemonEventHandler {
    /// Create a new DaemonEventHandler.
    pub fn new(agent_driver: Arc<AgentDriver>) -> Self {
        Self { agent_driver }
    }

    /// Process a task.
    pub async fn process(&self, task: Task) -> Result<(), DaemonError> {
        match task.task_type.as_str() {
            // Agent tasks
            "agent:execute" | "agent:subtask" | "agent:delayed" => {
                self.agent_driver
                    .process_task(task)
                    .await
                    .map_err(|e| DaemonError::Custom(format!("Agent error: {}", e)))?;
            }

            // System events
            "system:heartbeat" => {
                debug!("Heartbeat received");
            }
            "system:shutdown" => {
                info!("Shutdown event received");
            }
            "system:reload" => {
                info!("Reload event received");
                // TODO: Implement config reload
            }

            // Scheduler tasks
            "scheduler:job:due" => {
                info!("Scheduler job due: {:?}", task.payload);
                // Convert to agent:execute task
                if let Some(prompt) = task.payload.get("prompt").and_then(|v| v.as_str()) {
                    let agent = task
                        .payload
                        .get("agent")
                        .and_then(|v| v.as_str())
                        .unwrap_or("general");

                    let execute_event = Task::new(
                        "agent:execute",
                        serde_json::json!({
                            "agent": agent,
                            "prompt": prompt,
                        }),
                    )
                    .with_source(TaskSource::Scheduler)
                    .with_priority(TaskPriority::Normal);

                    self.agent_driver
                        .process_task(execute_event)
                        .await
                        .map_err(|e| DaemonError::Custom(format!("Agent error: {}", e)))?;
                }
            }

            // Trigger tasks
            "trigger:file:changed" | "trigger:webhook" => {
                info!("Trigger task: {:?}", task.payload);
                // Convert to agent:execute task
                if let Some(prompt) = task.payload.get("prompt").and_then(|v| v.as_str()) {
                    let agent = task
                        .payload
                        .get("agent")
                        .and_then(|v| v.as_str())
                        .unwrap_or("general");

                    let execute_event = Task::new(
                        "agent:execute",
                        serde_json::json!({
                            "agent": agent,
                            "prompt": prompt,
                        }),
                    )
                    .with_source(TaskSource::Custom("trigger".to_string()))
                    .with_priority(TaskPriority::High);

                    self.agent_driver
                        .process_task(execute_event)
                        .await
                        .map_err(|e| DaemonError::Custom(format!("Agent error: {}", e)))?;
                }
            }

            // Error tasks (for logging)
            "agent:error" => {
                warn!("Agent error: {:?}", task.payload);
            }

            _ => {
                debug!("Unhandled event type: {}", task.task_type);
            }
        }

        Ok(())
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
mod tests {
    use super::*;

    #[test]
    fn test_runloop_runner_builder() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());

        let runner = RunLoopDaemonBuilder::new()
            .provider_registry(provider_registry)
            .tool_registry(tool_registry)
            .default_agent("test-agent")
            .build()
            .unwrap();

        assert_eq!(runner.default_agent, "test-agent");
    }

    #[test]
    fn test_runloop_runner_builder_missing_registry() {
        let result = RunLoopDaemonBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_runloop_runner_creation() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());

        let runner =
            RunLoopRunner::new(provider_registry, tool_registry).with_default_agent("custom");

        assert_eq!(runner.default_agent, "custom");
    }
}
