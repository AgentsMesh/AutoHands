//! Job persistence store.

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, warn};

use super::definition::Job;
use crate::error::InterfaceError;

/// Job store trait for persistence.
#[async_trait]
pub trait JobStore: Send + Sync {
    /// Save a job.
    async fn save(&self, job: &Job) -> Result<(), InterfaceError>;

    /// Load a job by ID.
    async fn load(&self, id: &str) -> Result<Option<Job>, InterfaceError>;

    /// Load all jobs.
    async fn load_all(&self) -> Result<Vec<Job>, InterfaceError>;

    /// Delete a job.
    async fn delete(&self, id: &str) -> Result<(), InterfaceError>;

    /// Update job status.
    async fn update_status(&self, job: &Job) -> Result<(), InterfaceError>;
}

/// In-memory job store for testing.
pub struct MemoryJobStore {
    jobs: tokio::sync::RwLock<std::collections::HashMap<String, Job>>,
}

impl MemoryJobStore {
    /// Create a new memory store.
    pub fn new() -> Self {
        Self {
            jobs: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemoryJobStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JobStore for MemoryJobStore {
    async fn save(&self, job: &Job) -> Result<(), InterfaceError> {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.definition.id.clone(), job.clone());
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<Job>, InterfaceError> {
        let jobs = self.jobs.read().await;
        Ok(jobs.get(id).cloned())
    }

    async fn load_all(&self) -> Result<Vec<Job>, InterfaceError> {
        let jobs = self.jobs.read().await;
        Ok(jobs.values().cloned().collect())
    }

    async fn delete(&self, id: &str) -> Result<(), InterfaceError> {
        let mut jobs = self.jobs.write().await;
        jobs.remove(id);
        Ok(())
    }

    async fn update_status(&self, job: &Job) -> Result<(), InterfaceError> {
        self.save(job).await
    }
}

/// File system based job store for persistence.
pub struct FileJobStore {
    storage_path: PathBuf,
}

impl FileJobStore {
    /// Create a new file-based job store.
    pub async fn new(storage_path: impl Into<PathBuf>) -> Result<Self, InterfaceError> {
        let storage_path = storage_path.into();
        let jobs_dir = storage_path.join("jobs");

        fs::create_dir_all(&jobs_dir).await.map_err(|e| {
            InterfaceError::Custom(format!("Failed to create jobs directory: {}", e))
        })?;

        debug!("FileJobStore initialized at {:?}", storage_path);

        Ok(Self { storage_path })
    }

    fn jobs_dir(&self) -> PathBuf {
        self.storage_path.join("jobs")
    }

    fn job_path(&self, id: &str) -> PathBuf {
        self.jobs_dir().join(format!("{}.json", id))
    }

    fn sanitize_id(id: &str) -> String {
        id.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }
}

#[async_trait]
impl JobStore for FileJobStore {
    async fn save(&self, job: &Job) -> Result<(), InterfaceError> {
        let id = Self::sanitize_id(&job.definition.id);
        let path = self.job_path(&id);

        let content = serde_json::to_string_pretty(job)
            .map_err(|e| InterfaceError::Custom(format!("Failed to serialize job: {}", e)))?;

        fs::write(&path, content)
            .await
            .map_err(|e| InterfaceError::Custom(format!("Failed to write job file: {}", e)))?;

        debug!("Saved job '{}' to {:?}", job.definition.id, path);
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<Job>, InterfaceError> {
        let sanitized_id = Self::sanitize_id(id);
        let path = self.job_path(&sanitized_id);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| InterfaceError::Custom(format!("Failed to read job file: {}", e)))?;

        let job: Job = serde_json::from_str(&content)
            .map_err(|e| InterfaceError::Custom(format!("Failed to deserialize job: {}", e)))?;

        Ok(Some(job))
    }

    async fn load_all(&self) -> Result<Vec<Job>, InterfaceError> {
        let jobs_dir = self.jobs_dir();

        if !jobs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut jobs = Vec::new();
        let mut entries = fs::read_dir(&jobs_dir)
            .await
            .map_err(|e| InterfaceError::Custom(format!("Failed to read jobs directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            InterfaceError::Custom(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "json") {
                match fs::read_to_string(&path).await {
                    Ok(content) => match serde_json::from_str::<Job>(&content) {
                        Ok(job) => jobs.push(job),
                        Err(e) => {
                            warn!("Failed to deserialize job from {:?}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to read job file {:?}: {}", path, e);
                    }
                }
            }
        }

        debug!("Loaded {} jobs from {:?}", jobs.len(), jobs_dir);
        Ok(jobs)
    }

    async fn delete(&self, id: &str) -> Result<(), InterfaceError> {
        let sanitized_id = Self::sanitize_id(id);
        let path = self.job_path(&sanitized_id);

        if path.exists() {
            fs::remove_file(&path)
                .await
                .map_err(|e| InterfaceError::Custom(format!("Failed to delete job file: {}", e)))?;
            debug!("Deleted job '{}' from {:?}", id, path);
        }

        Ok(())
    }

    async fn update_status(&self, job: &Job) -> Result<(), InterfaceError> {
        self.save(job).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::JobDefinition;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_memory_job_store() {
        let store = MemoryJobStore::new();
        let def = JobDefinition::new("test-job", "0 * * * *", "agent", "prompt");
        let job = Job::new(def);

        store.save(&job).await.unwrap();

        let loaded = store.load("test-job").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().definition.id, "test-job");

        let all = store.load_all().await.unwrap();
        assert_eq!(all.len(), 1);

        store.delete("test-job").await.unwrap();
        let loaded = store.load("test-job").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_file_job_store_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileJobStore::new(temp_dir.path()).await.unwrap();

        let definition = JobDefinition::new("test-job", "0 * * * *", "test-agent", "Test prompt");
        let job = Job::new(definition);

        store.save(&job).await.unwrap();

        let loaded = store.load("test-job").await.unwrap();
        assert!(loaded.is_some());
        let loaded_job = loaded.unwrap();
        assert_eq!(loaded_job.definition.id, "test-job");
        assert_eq!(loaded_job.definition.agent, "test-agent");
    }

    #[tokio::test]
    async fn test_file_job_store_load_all() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileJobStore::new(temp_dir.path()).await.unwrap();

        for i in 0..3 {
            let definition = JobDefinition::new(
                format!("job-{}", i),
                "0 * * * *",
                "agent",
                format!("Prompt {}", i),
            );
            store.save(&Job::new(definition)).await.unwrap();
        }

        let jobs = store.load_all().await.unwrap();
        assert_eq!(jobs.len(), 3);
    }

    #[tokio::test]
    async fn test_file_job_store_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileJobStore::new(temp_dir.path()).await.unwrap();

        let definition = JobDefinition::new("to-delete", "0 * * * *", "agent", "Prompt");
        let job = Job::new(definition);

        store.save(&job).await.unwrap();
        assert!(store.load("to-delete").await.unwrap().is_some());

        store.delete("to-delete").await.unwrap();
        assert!(store.load("to-delete").await.unwrap().is_none());
    }

    #[test]
    fn test_sanitize_id() {
        assert_eq!(FileJobStore::sanitize_id("simple-job"), "simple-job");
        assert_eq!(
            FileJobStore::sanitize_id("job_with_underscore"),
            "job_with_underscore"
        );
        assert_eq!(
            FileJobStore::sanitize_id("job/with/slashes"),
            "job_with_slashes"
        );
        assert_eq!(
            FileJobStore::sanitize_id("job:with:colons"),
            "job_with_colons"
        );
    }
}
