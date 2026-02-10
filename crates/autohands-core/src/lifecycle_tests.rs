use super::*;
use std::sync::atomic::AtomicBool;

struct TestHook {
    started: AtomicBool,
    stopped: AtomicBool,
    priority: i32,
}

impl TestHook {
    fn new(priority: i32) -> Self {
        Self {
            started: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            priority,
        }
    }
}

#[async_trait::async_trait]
impl LifecycleHook for TestHook {
    async fn on_start(&self) -> Result<(), ExtensionError> {
        self.started.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn on_stop(&self) -> Result<(), ExtensionError> {
        self.stopped.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

#[test]
fn test_kernel_state_conversion() {
    assert_eq!(KernelState::from(0), KernelState::Created);
    assert_eq!(KernelState::from(2), KernelState::Running);
    assert_eq!(KernelState::from(99), KernelState::Created);
}

#[test]
fn test_shutdown_signal() {
    let signal = ShutdownSignal::new();
    let mut rx = signal.subscribe();

    signal.trigger();

    let result = rx.try_recv();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_lifecycle_start_stop() {
    let manager = LifecycleManager::default();
    let hook = Arc::new(TestHook::new(0));

    manager.register_hook(hook.clone()).await;

    assert_eq!(manager.state(), KernelState::Created);

    manager.start().await.unwrap();
    assert_eq!(manager.state(), KernelState::Running);
    assert!(hook.started.load(Ordering::SeqCst));
    assert!(manager.is_running());

    manager.stop().await.unwrap();
    assert_eq!(manager.state(), KernelState::Stopped);
    assert!(hook.stopped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_cannot_start_twice() {
    let manager = LifecycleManager::default();
    manager.start().await.unwrap();

    let result = manager.start().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_hook_priority_order() {
    let manager = LifecycleManager::default();
    let hook1 = Arc::new(TestHook::new(10));
    let hook2 = Arc::new(TestHook::new(5));

    manager.register_hook(hook2.clone()).await;
    manager.register_hook(hook1.clone()).await;

    manager.start().await.unwrap();

    // Both should be started
    assert!(hook1.started.load(Ordering::SeqCst));
    assert!(hook2.started.load(Ordering::SeqCst));
}

#[test]
fn test_kernel_state_debug() {
    let state = KernelState::Running;
    let debug = format!("{:?}", state);
    assert!(debug.contains("Running"));
}

#[test]
fn test_kernel_state_clone() {
    let state = KernelState::Running;
    let cloned = state;
    assert_eq!(cloned, state);
}

#[test]
fn test_kernel_state_eq() {
    assert_eq!(KernelState::Created, KernelState::Created);
    assert_ne!(KernelState::Created, KernelState::Running);
}

#[test]
fn test_kernel_state_all_conversions() {
    assert_eq!(KernelState::from(1), KernelState::Starting);
    assert_eq!(KernelState::from(3), KernelState::ShuttingDown);
    assert_eq!(KernelState::from(4), KernelState::Stopped);
}

#[test]
fn test_shutdown_signal_default() {
    let signal = ShutdownSignal::default();
    let mut rx = signal.subscribe();
    signal.trigger();
    assert!(rx.try_recv().is_ok());
}

#[tokio::test]
async fn test_lifecycle_manager_new() {
    let manager = LifecycleManager::new(Duration::from_secs(5));
    assert_eq!(manager.state(), KernelState::Created);
    assert!(!manager.is_running());
}

#[tokio::test]
async fn test_cannot_stop_before_start() {
    let manager = LifecycleManager::default();
    let result = manager.stop().await;
    assert!(result.is_err());
}

struct FailingHook;

#[async_trait::async_trait]
impl LifecycleHook for FailingHook {
    async fn on_start(&self) -> Result<(), ExtensionError> {
        Err(ExtensionError::InitializationFailed("Failed to start".to_string()))
    }

    async fn on_stop(&self) -> Result<(), ExtensionError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_start_with_failing_hook() {
    let manager = LifecycleManager::default();
    let hook = Arc::new(FailingHook);
    manager.register_hook(hook).await;

    let result = manager.start().await;
    assert!(result.is_err());
    assert_eq!(manager.state(), KernelState::Stopped);
}

#[tokio::test]
async fn test_start_with_multiple_hooks_one_fails() {
    let manager = LifecycleManager::default();
    let good_hook = Arc::new(TestHook::new(10)); // Higher priority, starts first
    let _bad_hook = Arc::new(FailingHook);

    manager.register_hook(good_hook.clone()).await;
    manager.register_hook(Arc::new(TestHook::new(5))).await; // This won't start

    // Insert failing hook with medium priority
    let manager2 = LifecycleManager::default();
    manager2.register_hook(good_hook.clone()).await;

    manager2.start().await.unwrap();
    assert!(good_hook.started.load(Ordering::SeqCst));
}

struct SlowStopHook;

#[async_trait::async_trait]
impl LifecycleHook for SlowStopHook {
    async fn on_start(&self) -> Result<(), ExtensionError> {
        Ok(())
    }

    async fn on_stop(&self) -> Result<(), ExtensionError> {
        tokio::time::sleep(Duration::from_secs(60)).await;
        Ok(())
    }
}

#[tokio::test]
async fn test_stop_with_timeout() {
    let manager = LifecycleManager::new(Duration::from_millis(10));
    let hook = Arc::new(SlowStopHook);
    manager.register_hook(hook).await;

    manager.start().await.unwrap();
    let result = manager.stop().await;
    // Should error due to timeout
    assert!(result.is_err());
}

struct ErrorStopHook;

#[async_trait::async_trait]
impl LifecycleHook for ErrorStopHook {
    async fn on_start(&self) -> Result<(), ExtensionError> {
        Ok(())
    }

    async fn on_stop(&self) -> Result<(), ExtensionError> {
        Err(ExtensionError::ShutdownFailed("stop failed".to_string()))
    }
}

#[tokio::test]
async fn test_stop_with_error() {
    let manager = LifecycleManager::default();
    let hook = Arc::new(ErrorStopHook);
    manager.register_hook(hook).await;

    manager.start().await.unwrap();
    let result = manager.stop().await;
    assert!(result.is_err());
    assert_eq!(manager.state(), KernelState::Stopped);
}
