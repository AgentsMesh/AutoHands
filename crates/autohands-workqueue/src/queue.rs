//! Priority queue implementation.

use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::config::QueueConfig;
use crate::error::QueueError;
use crate::task::{Task, TaskStatus};
use crate::store::{TaskStore, MemoryTaskStore};

/// Wrapper for priority queue ordering.
#[derive(Clone)]
struct PriorityTask(Task);

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for PriorityTask {}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier creation time
        match self.0.priority.cmp(&other.0.priority) {
            Ordering::Equal => other.0.created_at.cmp(&self.0.created_at),
            other => other,
        }
    }
}

/// Priority-based task queue.
pub struct TaskQueue {
    config: QueueConfig,
    store: Arc<dyn TaskStore>,
    queue: RwLock<BinaryHeap<PriorityTask>>,
    dead_letter: RwLock<Vec<Task>>,
}

impl TaskQueue {
    /// Create a new task queue.
    pub fn new(config: QueueConfig) -> Self {
        Self {
            config,
            store: Arc::new(MemoryTaskStore::new()),
            queue: RwLock::new(BinaryHeap::new()),
            dead_letter: RwLock::new(Vec::new()),
        }
    }

    /// Create a queue with a custom store.
    pub fn with_store(config: QueueConfig, store: Arc<dyn TaskStore>) -> Self {
        Self {
            config,
            store,
            queue: RwLock::new(BinaryHeap::new()),
            dead_letter: RwLock::new(Vec::new()),
        }
    }

    /// Enqueue a task.
    pub async fn enqueue(&self, task: Task) -> Result<(), QueueError> {
        // Check queue size limit
        if self.config.max_queue_size > 0 {
            let queue = self.queue.read().await;
            if queue.len() as u64 >= self.config.max_queue_size {
                return Err(QueueError::QueueFull);
            }
        }

        self.store.save(&task).await?;

        let mut queue = self.queue.write().await;
        debug!("Enqueueing task: {} (priority: {:?})", task.id, task.priority);
        queue.push(PriorityTask(task));

        Ok(())
    }

    /// Dequeue the highest priority ready task.
    pub async fn dequeue(&self) -> Result<Option<Task>, QueueError> {
        let mut queue = self.queue.write().await;

        // Find the first ready task
        let mut temp = Vec::new();
        let mut result = None;

        while let Some(pt) = queue.pop() {
            if pt.0.is_ready() {
                result = Some(pt.0);
                break;
            } else {
                temp.push(pt);
            }
        }

        // Put back tasks that weren't ready
        for pt in temp {
            queue.push(pt);
        }

        if let Some(ref task) = result {
            debug!("Dequeued task: {}", task.id);
        }

        Ok(result)
    }

    /// Get queue length.
    pub async fn len(&self) -> usize {
        self.queue.read().await.len()
    }

    /// Check if queue is empty.
    pub async fn is_empty(&self) -> bool {
        self.queue.read().await.is_empty()
    }

    /// Move a task to the dead letter queue.
    pub async fn move_to_dead_letter(&self, mut task: Task) -> Result<(), QueueError> {
        if !self.config.dead_letter_queue_enabled {
            return Ok(());
        }

        task.status = TaskStatus::DeadLetter;
        self.store.update(&task).await?;

        let mut dlq = self.dead_letter.write().await;
        info!("Moving task to dead letter queue: {}", task.id);
        dlq.push(task);

        Ok(())
    }

    /// Get dead letter queue contents.
    pub async fn dead_letter_queue(&self) -> Vec<Task> {
        self.dead_letter.read().await.clone()
    }

    /// Retry a task (increment retry count and re-enqueue).
    pub async fn retry(&self, mut task: Task, error: &str) -> Result<bool, QueueError> {
        task.retry_count += 1;
        task.last_error = Some(error.to_string());

        if !task.can_retry() {
            self.move_to_dead_letter(task).await?;
            return Ok(false);
        }

        task.status = TaskStatus::Pending;
        self.store.update(&task).await?;

        let mut queue = self.queue.write().await;
        debug!("Retrying task: {} (attempt {})", task.id, task.retry_count);
        queue.push(PriorityTask(task));

        Ok(true)
    }

    /// Load pending tasks from store.
    pub async fn load_from_store(&self) -> Result<(), QueueError> {
        let tasks = self.store.load_pending().await?;
        let mut queue = self.queue.write().await;

        for task in tasks {
            queue.push(PriorityTask(task));
        }

        info!("Loaded {} tasks from store", queue.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskPriority;

    #[tokio::test]
    async fn test_queue_enqueue_dequeue() {
        let queue = TaskQueue::new(QueueConfig::default());
        let task = Task::new("test", "general", "payload");

        queue.enqueue(task.clone()).await.unwrap();
        assert_eq!(queue.len().await, 1);

        let dequeued = queue.dequeue().await.unwrap();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, task.id);
        assert_eq!(queue.len().await, 0);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let queue = TaskQueue::new(QueueConfig::default());

        let low = Task::new("low", "general", "").with_priority(TaskPriority::Low);
        let high = Task::new("high", "general", "").with_priority(TaskPriority::High);
        let normal = Task::new("normal", "general", "").with_priority(TaskPriority::Normal);

        queue.enqueue(low).await.unwrap();
        queue.enqueue(high).await.unwrap();
        queue.enqueue(normal).await.unwrap();

        let first = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(first.name, "high");

        let second = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(second.name, "normal");

        let third = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(third.name, "low");
    }

    #[tokio::test]
    async fn test_queue_size_limit() {
        let config = QueueConfig {
            max_queue_size: 2,
            ..Default::default()
        };
        let queue = TaskQueue::new(config);

        queue.enqueue(Task::new("1", "general", "")).await.unwrap();
        queue.enqueue(Task::new("2", "general", "")).await.unwrap();

        let result = queue.enqueue(Task::new("3", "general", "")).await;
        assert!(matches!(result, Err(QueueError::QueueFull)));
    }

    #[tokio::test]
    async fn test_retry() {
        let queue = TaskQueue::new(QueueConfig::default());
        // max_retries = 3 means can retry up to 3 times (retry_count < max_retries)
        let mut task = Task::new("test", "general", "").with_max_retries(3);
        task.status = TaskStatus::Failed;

        // First retry (retry_count becomes 1)
        assert!(queue.retry(task.clone(), "error 1").await.unwrap());
        assert_eq!(queue.len().await, 1);

        // Dequeue and retry again (retry_count becomes 2)
        let task = queue.dequeue().await.unwrap().unwrap();
        assert!(queue.retry(task.clone(), "error 2").await.unwrap());

        // Dequeue and retry again (retry_count becomes 3, which equals max_retries, so DLQ)
        let task = queue.dequeue().await.unwrap().unwrap();
        assert!(!queue.retry(task, "error 3").await.unwrap());
        assert!(queue.is_empty().await);

        let dlq = queue.dead_letter_queue().await;
        assert_eq!(dlq.len(), 1);
    }
}
