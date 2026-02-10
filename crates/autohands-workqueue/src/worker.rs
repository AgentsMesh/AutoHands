//! Worker pool for task execution.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use async_trait::async_trait;
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

use crate::config::QueueConfig;
use crate::error::QueueError;
use crate::task::{Task, TaskStatus};
use crate::queue::TaskQueue;

/// Task handler trait.
#[async_trait]
pub trait TaskHandler: Send + Sync {
    /// Execute a task.
    async fn handle(&self, task: &Task) -> Result<(), QueueError>;
}

/// A single worker.
pub struct Worker {
    id: u32,
    running: AtomicBool,
    tasks_completed: AtomicU64,
    tasks_failed: AtomicU64,
}

impl Worker {
    /// Create a new worker.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            running: AtomicBool::new(false),
            tasks_completed: AtomicU64::new(0),
            tasks_failed: AtomicU64::new(0),
        }
    }

    /// Get worker ID.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Check if worker is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get completed task count.
    pub fn tasks_completed(&self) -> u64 {
        self.tasks_completed.load(Ordering::SeqCst)
    }

    /// Get failed task count.
    pub fn tasks_failed(&self) -> u64 {
        self.tasks_failed.load(Ordering::SeqCst)
    }

    /// Process a task.
    pub async fn process<H: TaskHandler>(
        &self,
        mut task: Task,
        handler: &H,
        queue: &TaskQueue,
    ) -> Result<(), QueueError> {
        self.running.store(true, Ordering::SeqCst);
        debug!("Worker {} processing task {}", self.id, task.id);

        task.status = TaskStatus::Running;

        match handler.handle(&task).await {
            Ok(()) => {
                task.status = TaskStatus::Completed;
                self.tasks_completed.fetch_add(1, Ordering::SeqCst);
                debug!("Worker {} completed task {}", self.id, task.id);
            }
            Err(e) => {
                task.status = TaskStatus::Failed;
                self.tasks_failed.fetch_add(1, Ordering::SeqCst);
                error!("Worker {} failed task {}: {}", self.id, task.id, e);

                // Retry the task
                queue.retry(task, &e.to_string()).await?;
            }
        }

        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }
}

/// Worker pool for concurrent task execution.
pub struct WorkerPool {
    config: QueueConfig,
    semaphore: Arc<Semaphore>,
    running: Arc<AtomicBool>,
    total_processed: Arc<AtomicU64>,
}

impl WorkerPool {
    /// Create a new worker pool.
    pub fn new(config: QueueConfig) -> Self {
        let permits = config.max_workers as usize;
        Self {
            config,
            semaphore: Arc::new(Semaphore::new(permits)),
            running: Arc::new(AtomicBool::new(false)),
            total_processed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Start the worker pool.
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
        info!("Worker pool started with {} workers", self.config.max_workers);
    }

    /// Stop the worker pool.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        info!("Worker pool stopped");
    }

    /// Check if pool is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get total processed task count.
    pub fn total_processed(&self) -> u64 {
        self.total_processed.load(Ordering::SeqCst)
    }

    /// Get number of available workers.
    pub fn available_workers(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Submit a task for execution.
    pub async fn submit<H: TaskHandler + 'static>(
        &self,
        task: Task,
        handler: Arc<H>,
        queue: Arc<TaskQueue>,
    ) -> Result<(), QueueError> {
        if !self.is_running() {
            return Err(QueueError::WorkerError("Pool is not running".to_string()));
        }

        let permit = self.semaphore.clone().acquire_owned().await
            .map_err(|e| QueueError::WorkerError(e.to_string()))?;

        let total_processed = self.total_processed.clone();
        let worker_id = self.config.max_workers - self.available_workers() as u32;

        tokio::spawn(async move {
            let worker = Worker::new(worker_id);
            let result = worker.process(task, handler.as_ref(), queue.as_ref()).await;

            if result.is_ok() {
                total_processed.fetch_add(1, Ordering::SeqCst);
            }

            drop(permit);
        });

        Ok(())
    }

    /// Run the pool in a loop, processing tasks from the queue.
    pub async fn run_loop<H: TaskHandler + 'static>(
        self: Arc<Self>,
        queue: Arc<TaskQueue>,
        handler: Arc<H>,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) {
        self.start();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Worker pool shutting down");
                    break;
                }
                _ = async {
                    if let Ok(Some(task)) = queue.dequeue().await {
                        if let Err(e) = self.submit(task, handler.clone(), queue.clone()).await {
                            error!("Failed to submit task: {}", e);
                        }
                    } else {
                        // No tasks available, wait a bit
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                } => {}
            }
        }

        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    #[async_trait]
    impl TaskHandler for TestHandler {
        async fn handle(&self, _task: &Task) -> Result<(), QueueError> {
            Ok(())
        }
    }

    #[test]
    fn test_worker_new() {
        let worker = Worker::new(1);
        assert_eq!(worker.id(), 1);
        assert!(!worker.is_running());
        assert_eq!(worker.tasks_completed(), 0);
    }

    #[tokio::test]
    async fn test_worker_process_success() {
        let worker = Worker::new(1);
        let task = Task::new("test", "general", "payload");
        let queue = TaskQueue::new(QueueConfig::default());
        let handler = TestHandler;

        worker.process(task, &handler, &queue).await.unwrap();
        assert_eq!(worker.tasks_completed(), 1);
        assert_eq!(worker.tasks_failed(), 0);
    }

    #[test]
    fn test_worker_pool_new() {
        let config = QueueConfig {
            max_workers: 4,
            ..Default::default()
        };
        let pool = WorkerPool::new(config);

        assert!(!pool.is_running());
        assert_eq!(pool.available_workers(), 4);
    }

    #[tokio::test]
    async fn test_worker_pool_submit() {
        let pool = WorkerPool::new(QueueConfig::default());
        pool.start();

        let task = Task::new("test", "general", "payload");
        let handler = Arc::new(TestHandler);
        let queue = Arc::new(TaskQueue::new(QueueConfig::default()));

        pool.submit(task, handler, queue).await.unwrap();

        // Wait for task to complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        pool.stop();
    }
}
