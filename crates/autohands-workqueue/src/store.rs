//! Task persistence store.

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::QueueError;
use crate::task::{Task, TaskStatus};

/// Task store trait for persistence.
#[async_trait]
pub trait TaskStore: Send + Sync {
    /// Save a task.
    async fn save(&self, task: &Task) -> Result<(), QueueError>;

    /// Load a task by ID.
    async fn load(&self, id: &uuid::Uuid) -> Result<Option<Task>, QueueError>;

    /// Load all pending tasks.
    async fn load_pending(&self) -> Result<Vec<Task>, QueueError>;

    /// Delete a task.
    async fn delete(&self, id: &uuid::Uuid) -> Result<(), QueueError>;

    /// Update task status.
    async fn update(&self, task: &Task) -> Result<(), QueueError>;
}

/// In-memory task store for testing.
pub struct MemoryTaskStore {
    tasks: tokio::sync::RwLock<std::collections::HashMap<uuid::Uuid, Task>>,
}

impl MemoryTaskStore {
    /// Create a new memory store.
    pub fn new() -> Self {
        Self {
            tasks: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemoryTaskStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskStore for MemoryTaskStore {
    async fn save(&self, task: &Task) -> Result<(), QueueError> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id, task.clone());
        Ok(())
    }

    async fn load(&self, id: &uuid::Uuid) -> Result<Option<Task>, QueueError> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(id).cloned())
    }

    async fn load_pending(&self) -> Result<Vec<Task>, QueueError> {
        use crate::task::TaskStatus;
        let tasks = self.tasks.read().await;
        Ok(tasks.values()
            .filter(|t| t.status == TaskStatus::Pending)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &uuid::Uuid) -> Result<(), QueueError> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(id);
        Ok(())
    }

    async fn update(&self, task: &Task) -> Result<(), QueueError> {
        self.save(task).await
    }
}

/// File system based task store for persistence.
///
/// Tasks are stored as individual JSON files organized by status:
/// ```text
/// {storage_path}/
/// └── tasks/
///     ├── pending/
///     │   └── {uuid}.json
///     ├── running/
///     │   └── {uuid}.json
///     ├── completed/
///     │   └── {uuid}.json
///     ├── failed/
///     │   └── {uuid}.json
///     └── dead_letter/
///         └── {uuid}.json
/// ```
pub struct FileTaskStore {
    /// Base storage path.
    storage_path: PathBuf,
}

impl FileTaskStore {
    /// Create a new file-based task store.
    ///
    /// # Arguments
    /// * `storage_path` - Base directory for storing task files
    pub async fn new(storage_path: impl Into<PathBuf>) -> Result<Self, QueueError> {
        let storage_path = storage_path.into();
        let tasks_dir = storage_path.join("tasks");

        // Create all status directories
        for status_dir in &["pending", "running", "completed", "failed", "dead_letter", "cancelled"] {
            let dir = tasks_dir.join(status_dir);
            fs::create_dir_all(&dir).await.map_err(|e| {
                QueueError::Database(format!("Failed to create {} directory: {}", status_dir, e))
            })?;
        }

        debug!("FileTaskStore initialized at {:?}", storage_path);

        Ok(Self { storage_path })
    }

    /// Get the tasks directory path.
    fn tasks_dir(&self) -> PathBuf {
        self.storage_path.join("tasks")
    }

    /// Get the directory for a specific status.
    fn status_dir(&self, status: TaskStatus) -> PathBuf {
        let status_name = match status {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::DeadLetter => "dead_letter",
            TaskStatus::Cancelled => "cancelled",
        };
        self.tasks_dir().join(status_name)
    }

    /// Get the file path for a task in a specific status directory.
    fn task_path(&self, id: &Uuid, status: TaskStatus) -> PathBuf {
        self.status_dir(status).join(format!("{}.json", id))
    }

    /// Find the current location of a task file.
    async fn find_task_file(&self, id: &Uuid) -> Option<(PathBuf, TaskStatus)> {
        let statuses = [
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::DeadLetter,
            TaskStatus::Cancelled,
        ];

        for status in statuses {
            let path = self.task_path(id, status);
            if path.exists() {
                return Some((path, status));
            }
        }
        None
    }

}

#[async_trait]
impl TaskStore for FileTaskStore {
    async fn save(&self, task: &Task) -> Result<(), QueueError> {
        // First, find and remove any existing file for this task
        if let Some((old_path, old_status)) = self.find_task_file(&task.id).await {
            if old_status != task.status {
                fs::remove_file(&old_path).await.ok(); // Ignore errors on cleanup
            }
        }

        let path = self.task_path(&task.id, task.status);

        let content = serde_json::to_string_pretty(task).map_err(|e| {
            QueueError::Database(format!("Failed to serialize task: {}", e))
        })?;

        fs::write(&path, content).await.map_err(|e| {
            QueueError::Database(format!("Failed to write task file: {}", e))
        })?;

        debug!("Saved task '{}' to {:?}", task.id, path);
        Ok(())
    }

    async fn load(&self, id: &Uuid) -> Result<Option<Task>, QueueError> {
        let Some((path, _)) = self.find_task_file(id).await else {
            return Ok(None);
        };

        let content = fs::read_to_string(&path).await.map_err(|e| {
            QueueError::Database(format!("Failed to read task file: {}", e))
        })?;

        let task: Task = serde_json::from_str(&content).map_err(|e| {
            QueueError::Database(format!("Failed to deserialize task: {}", e))
        })?;

        Ok(Some(task))
    }

    async fn load_pending(&self) -> Result<Vec<Task>, QueueError> {
        let pending_dir = self.status_dir(TaskStatus::Pending);

        if !pending_dir.exists() {
            return Ok(Vec::new());
        }

        let mut tasks = Vec::new();
        let mut entries = fs::read_dir(&pending_dir).await.map_err(|e| {
            QueueError::Database(format!("Failed to read pending directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            QueueError::Database(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                match fs::read_to_string(&path).await {
                    Ok(content) => {
                        match serde_json::from_str::<Task>(&content) {
                            Ok(task) => tasks.push(task),
                            Err(e) => {
                                warn!("Failed to deserialize task from {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read task file {:?}: {}", path, e);
                    }
                }
            }
        }

        // Sort by priority (highest first) and then by creation time (oldest first)
        tasks.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });

        debug!("Loaded {} pending tasks", tasks.len());
        Ok(tasks)
    }

    async fn delete(&self, id: &Uuid) -> Result<(), QueueError> {
        if let Some((path, _)) = self.find_task_file(id).await {
            fs::remove_file(&path).await.map_err(|e| {
                QueueError::Database(format!("Failed to delete task file: {}", e))
            })?;
            debug!("Deleted task '{}'", id);
        }

        Ok(())
    }

    async fn update(&self, task: &Task) -> Result<(), QueueError> {
        self.save(task).await
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
