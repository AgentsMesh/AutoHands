//! End-to-end integration tests for RunLoop architecture.
//!
//! These tests verify the complete flow from event injection to agent execution.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::mpsc;

use autohands_runloop::{
    AgentDriver, AgentEventHandler, AgentTaskInjector, AgentSource0, AgentResult,
    TaskPriority, TaskQueue, TaskQueueConfig, TaskSource, HttpTaskInjector,
    RunLoop, RunLoopConfig, Task, RunLoopMode, RunLoopObserver, RunLoopPhase,
    RunLoopResult, RunLoopRunResult, Source0, TimerBuilder, WakeupSignal,
    WebSocketSource1,
};

// ============================================================================
// Test Helpers
// ============================================================================

/// Test event handler that tracks execution count.
struct TestEventHandler {
    execute_count: Arc<AtomicU32>,
    subtask_count: Arc<AtomicU32>,
    delayed_count: Arc<AtomicU32>,
}

impl TestEventHandler {
    fn new() -> (Self, Arc<AtomicU32>, Arc<AtomicU32>, Arc<AtomicU32>) {
        let execute_count = Arc::new(AtomicU32::new(0));
        let subtask_count = Arc::new(AtomicU32::new(0));
        let delayed_count = Arc::new(AtomicU32::new(0));

        (
            Self {
                execute_count: execute_count.clone(),
                subtask_count: subtask_count.clone(),
                delayed_count: delayed_count.clone(),
            },
            execute_count,
            subtask_count,
            delayed_count,
        )
    }
}

#[async_trait]
impl AgentEventHandler for TestEventHandler {
    async fn handle_execute(
        &self,
        event: &Task,
        _injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        self.execute_count.fetch_add(1, Ordering::SeqCst);

        let prompt = event
            .payload
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(AgentResult::completed(format!("Executed: {}", prompt)))
    }

    async fn handle_subtask(
        &self,
        _event: &Task,
        _injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        self.subtask_count.fetch_add(1, Ordering::SeqCst);
        Ok(AgentResult::completed("Subtask completed"))
    }

    async fn handle_delayed(
        &self,
        _event: &Task,
        _injector: &AgentTaskInjector,
    ) -> RunLoopResult<AgentResult> {
        self.delayed_count.fetch_add(1, Ordering::SeqCst);
        Ok(AgentResult::completed("Delayed task completed"))
    }
}

/// Test observer that tracks phase notifications.
struct TestObserver {
    entry_count: Arc<AtomicU32>,
    exit_count: Arc<AtomicU32>,
    before_waiting_count: Arc<AtomicU32>,
}

impl TestObserver {
    fn new() -> (Self, Arc<AtomicU32>, Arc<AtomicU32>, Arc<AtomicU32>) {
        let entry_count = Arc::new(AtomicU32::new(0));
        let exit_count = Arc::new(AtomicU32::new(0));
        let before_waiting_count = Arc::new(AtomicU32::new(0));

        (
            Self {
                entry_count: entry_count.clone(),
                exit_count: exit_count.clone(),
                before_waiting_count: before_waiting_count.clone(),
            },
            entry_count,
            exit_count,
            before_waiting_count,
        )
    }
}

#[async_trait]
impl RunLoopObserver for TestObserver {
    fn activities(&self) -> u32 {
        RunLoopPhase::Entry as u32
            | RunLoopPhase::Exit as u32
            | RunLoopPhase::BeforeWaiting as u32
    }

    async fn on_phase(&self, phase: RunLoopPhase, _run_loop: &RunLoop) {
        match phase {
            RunLoopPhase::Entry => {
                self.entry_count.fetch_add(1, Ordering::SeqCst);
            }
            RunLoopPhase::Exit => {
                self.exit_count.fetch_add(1, Ordering::SeqCst);
            }
            RunLoopPhase::BeforeWaiting => {
                self.before_waiting_count.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

/// Test: RunLoop basic lifecycle with observers.
#[tokio::test]
async fn test_runloop_lifecycle_with_observers() {
    let run_loop = Arc::new(RunLoop::default());

    // Add test observer
    let (observer, entry_count, exit_count, _before_waiting) = TestObserver::new();
    run_loop.add_observer("test", Arc::new(observer)).await;

    // Stop after a short delay
    let run_loop_clone = run_loop.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        run_loop_clone.stop();
    });

    // Run
    let result = run_loop
        .run_in_mode(RunLoopMode::Default, Duration::from_secs(5))
        .await;

    assert!(matches!(result, Ok(RunLoopRunResult::Stopped)));
    assert_eq!(entry_count.load(Ordering::SeqCst), 1);
    assert_eq!(exit_count.load(Ordering::SeqCst), 1);
}

/// Test: AgentDriver processes events correctly.
#[tokio::test]
async fn test_agent_driver_event_processing() {
    let run_loop = Arc::new(RunLoop::default());
    let agent_source = Arc::new(AgentSource0::new("test-agent"));
    let config = RunLoopConfig::default();

    // Create handler with counters
    let (handler, execute_count, subtask_count, delayed_count) = TestEventHandler::new();

    let agent_driver = Arc::new(
        AgentDriver::new(run_loop.clone(), agent_source.clone(), config)
            .with_handler(Arc::new(handler)),
    );
    agent_driver.start();

    // Inject test events
    let execute_event = Task::new("agent:execute", json!({"prompt": "test task"}))
        .with_source(TaskSource::User)
        .with_priority(TaskPriority::High);

    let subtask_event = Task::new("agent:subtask", json!({"task": "sub work"}))
        .with_source(TaskSource::Agent)
        .with_priority(TaskPriority::Normal);

    let delayed_event = Task::new("agent:delayed", json!({"task": "delayed work"}))
        .with_source(TaskSource::Timer)
        .with_priority(TaskPriority::Low);

    // Process events
    agent_driver.process_task(execute_event).await.unwrap();
    agent_driver.process_task(subtask_event).await.unwrap();
    agent_driver.process_task(delayed_event).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify counts
    assert_eq!(execute_count.load(Ordering::SeqCst), 1);
    assert_eq!(subtask_count.load(Ordering::SeqCst), 1);
    assert_eq!(delayed_count.load(Ordering::SeqCst), 1);

    agent_driver.stop();
}

/// Test: Event injection via AgentSource0 (self-driving).
#[tokio::test]
async fn test_agent_self_driving_events() {
    let run_loop = Arc::new(RunLoop::default());
    let agent_source = Arc::new(AgentSource0::new("self-drive"));

    // Register source
    run_loop.add_source0(agent_source.clone()).await;

    // Inject event through the source
    let event = Task::new("agent:execute", json!({"prompt": "self-driven task"}))
        .with_source(TaskSource::Agent);

    agent_source.inject(event, &run_loop);

    // Verify source is signaled
    assert!(agent_source.is_signaled());

    // Verify event is in queue after perform (need Source0 trait in scope)
    let events = agent_source.perform().await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].task_type, "agent:execute");
}

/// Test: Timer generates events at intervals.
#[tokio::test]
async fn test_timer_event_generation() {
    let run_loop = Arc::new(RunLoop::default());

    // Create a repeating timer
    let timer = TimerBuilder::new()
        .id("test-timer")
        .interval(Duration::from_millis(50))
        .repeating()
        .task_type("timer:tick")
        .priority(TaskPriority::Low)
        .build(run_loop.clone());

    // Run loop for a short time
    let run_loop_clone = run_loop.clone();
    tokio::spawn(async move {
        // Wait for some timer ticks
        tokio::time::sleep(Duration::from_millis(200)).await;
        run_loop_clone.stop();
    });

    // Process events
    let _ = run_loop
        .run_in_mode(RunLoopMode::Default, Duration::from_secs(1))
        .await;

    timer.cancel();

    // Timer should have generated some events
    // Note: actual count depends on timing, just verify it ran
    let snapshot = run_loop.metrics().snapshot();
    assert!(snapshot.iterations > 0);
}

/// Test: HttpTaskInjector injects events correctly.
#[tokio::test]
async fn test_http_task_injector() {
    let (tx, _rx) = mpsc::channel::<WakeupSignal>(16);
    let config = TaskQueueConfig::default();
    let queue = Arc::new(TaskQueue::new(config, 100));

    let injector = HttpTaskInjector::new(tx, queue.clone());

    // Inject a task
    injector
        .inject_task("test http task", "session-123", Some("coder".to_string()))
        .await
        .unwrap();

    // Verify event is in queue
    let event = queue.dequeue().await.unwrap();
    assert_eq!(event.task_type, "agent:execute");
    assert_eq!(event.payload["prompt"], "test http task");
    assert_eq!(event.payload["session_id"], "session-123");
    assert_eq!(event.payload["agent"], "coder");
    assert_eq!(event.source, TaskSource::User);
}

/// Test: WebSocketSource1 handles chat messages.
#[tokio::test]
async fn test_websocket_source1_chat() {
    let ws_source = WebSocketSource1::new("test-ws");
    let (receiver, sender) = ws_source.create_receiver();

    // Send a chat message
    sender
        .send_chat(Some("sess-1".to_string()), "Hello from WebSocket", "conn-1")
        .await
        .unwrap();

    // Receive and handle
    let msg = receiver.receiver.lock().await.recv().await.unwrap();

    // Verify message content
    assert_eq!(msg.payload["type"], "chat");
    assert_eq!(msg.payload["content"], "Hello from WebSocket");
    assert_eq!(msg.payload["session_id"], "sess-1");
    assert_eq!(msg.payload["connection_id"], "conn-1");
}

/// Test: Event priority ordering.
#[tokio::test]
async fn test_event_priority_ordering() {
    let config = TaskQueueConfig::default();
    let queue = TaskQueue::new(config, 100);

    // Enqueue events with different priorities (in reverse order)
    let low_event = Task::new("test:low", json!({})).with_priority(TaskPriority::Low);

    let normal_event =
        Task::new("test:normal", json!({})).with_priority(TaskPriority::Normal);

    let high_event = Task::new("test:high", json!({})).with_priority(TaskPriority::High);

    let critical_event =
        Task::new("test:critical", json!({})).with_priority(TaskPriority::Critical);

    // Enqueue in wrong order
    queue.enqueue(low_event).await.unwrap();
    queue.enqueue(normal_event).await.unwrap();
    queue.enqueue(high_event).await.unwrap();
    queue.enqueue(critical_event).await.unwrap();

    // Dequeue should be in priority order
    assert_eq!(queue.dequeue().await.unwrap().task_type, "test:critical");
    assert_eq!(queue.dequeue().await.unwrap().task_type, "test:high");
    assert_eq!(queue.dequeue().await.unwrap().task_type, "test:normal");
    assert_eq!(queue.dequeue().await.unwrap().task_type, "test:low");
}

/// Test: Delayed event scheduling.
#[tokio::test]
async fn test_delayed_event_scheduling() {
    let config = TaskQueueConfig::default();
    let queue = TaskQueue::new(config, 100);

    // Create a delayed event (100ms in future)
    let delayed_event = Task::new("test:delayed", json!({}))
        .with_scheduled_at(chrono::Utc::now() + chrono::Duration::milliseconds(100));

    // Create an immediate event
    let immediate_event = Task::new("test:immediate", json!({}));

    // Enqueue delayed first, then immediate
    queue.enqueue(delayed_event).await.unwrap();
    queue.enqueue(immediate_event).await.unwrap();

    // Immediate should be available now
    assert_eq!(
        queue.dequeue().await.unwrap().task_type,
        "test:immediate"
    );

    // Delayed should not be available yet
    assert!(queue.dequeue().await.is_none());

    // Wait for delay
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Promote delayed events
    queue.promote_delayed().await;

    // Now delayed should be available
    assert_eq!(queue.dequeue().await.unwrap().task_type, "test:delayed");
}

/// Test: Event chain limiting.
#[tokio::test]
async fn test_event_chain_limiting() {
    let config = TaskQueueConfig::default();
    let queue = TaskQueue::new(config, 5); // Low limit for testing

    let correlation_id = "test-chain-123";

    // Enqueue events up to limit
    for i in 0..5 {
        let event = Task::new("test:chain", json!({"index": i}))
            .with_correlation_id(correlation_id);
        queue.enqueue(event).await.unwrap();
    }

    // Next event should fail due to chain limit
    let overflow_event =
        Task::new("test:overflow", json!({})).with_correlation_id(correlation_id);

    let result = queue.enqueue(overflow_event).await;
    assert!(result.is_err());
}

/// Test: Complete flow - HTTP injection to event processing.
#[tokio::test]
async fn test_complete_http_to_processing_flow() {
    let run_loop = Arc::new(RunLoop::default());
    let agent_source = Arc::new(AgentSource0::new("flow-test"));
    let config = RunLoopConfig::default();

    // Set up event handler
    let (handler, execute_count, _, _) = TestEventHandler::new();
    let agent_driver = Arc::new(
        AgentDriver::new(run_loop.clone(), agent_source.clone(), config)
            .with_handler(Arc::new(handler)),
    );
    agent_driver.start();

    // Create HTTP injector
    let injector =
        HttpTaskInjector::new(run_loop.wakeup_sender(), run_loop.task_queue());

    // Inject task via HTTP
    injector
        .inject_task("HTTP task", "http-session", None)
        .await
        .unwrap();

    // Verify event is in queue
    assert_eq!(run_loop.pending_task_count().await, 1);

    // Process the event through agent driver
    let event = run_loop.task_queue().dequeue().await.unwrap();
    agent_driver.process_task(event).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify execution
    assert_eq!(execute_count.load(Ordering::SeqCst), 1);

    agent_driver.stop();
}

/// Test: Multiple modes isolation.
#[tokio::test]
async fn test_mode_isolation() {
    let run_loop = Arc::new(RunLoop::default());

    // Add observer that only works in Default mode
    let (observer, entry_count, _, _) = TestObserver::new();
    run_loop
        .add_mode_observer(&RunLoopMode::Default, "default-only", Arc::new(observer))
        .await;

    // Run in Default mode briefly
    let run_loop_clone = run_loop.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        run_loop_clone.stop();
    });

    run_loop
        .run_in_mode(RunLoopMode::Default, Duration::from_secs(1))
        .await
        .unwrap();

    // Observer should have been called
    assert_eq!(entry_count.load(Ordering::SeqCst), 1);
}

/// Test: Metrics are collected correctly.
#[tokio::test]
async fn test_metrics_collection() {
    let run_loop = Arc::new(RunLoop::default());

    // Inject some events
    for i in 0..5 {
        let event = Task::new("test:metrics", json!({"index": i}));
        run_loop.inject_task(event).await.unwrap();
    }

    // Run briefly
    let run_loop_clone = run_loop.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        run_loop_clone.stop();
    });

    run_loop
        .run_in_mode(RunLoopMode::Default, Duration::from_secs(1))
        .await
        .unwrap();

    // Check metrics using snapshot
    let snapshot = run_loop.metrics().snapshot();
    assert!(snapshot.iterations > 0);
    assert!(snapshot.events_enqueued >= 5);
}

/// Test: Graceful shutdown with pending events.
#[tokio::test]
async fn test_graceful_shutdown() {
    let run_loop = Arc::new(RunLoop::default());

    // Inject events
    for i in 0..10 {
        let event = Task::new("test:shutdown", json!({"index": i}));
        run_loop.inject_task(event).await.unwrap();
    }

    // Stop immediately
    run_loop.stop();

    // Run should exit cleanly
    let result = run_loop
        .run_in_mode(RunLoopMode::Default, Duration::from_secs(1))
        .await;

    assert!(matches!(result, Ok(RunLoopRunResult::Stopped)));

    // Some events may still be pending (not processed due to immediate stop)
    // This is expected behavior for graceful shutdown
}
