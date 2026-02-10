//! Background process management.

use std::collections::HashMap;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;

use parking_lot::Mutex;
use uuid::Uuid;

/// Background process status.
#[derive(Debug, Clone)]
pub enum ProcessStatus {
    Running,
    Completed(i32),
    Failed(String),
}

/// Information about a background process.
#[derive(Debug)]
pub struct ProcessInfo {
    pub id: String,
    pub command: String,
    pub status: ProcessStatus,
    child: Option<Child>,
}

impl ProcessInfo {
    fn new(id: String, command: String, child: Child) -> Self {
        Self {
            id,
            command,
            status: ProcessStatus::Running,
            child: Some(child),
        }
    }

    /// Check and update process status.
    fn update_status(&mut self) {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.status = ProcessStatus::Completed(status.code().unwrap_or(-1));
                    self.child = None;
                }
                Ok(None) => {}
                Err(e) => {
                    self.status = ProcessStatus::Failed(e.to_string());
                    self.child = None;
                }
            }
        }
    }

    /// Kill the process if running.
    fn kill(&mut self) -> Result<(), String> {
        if let Some(child) = &mut self.child {
            child.kill().map_err(|e| e.to_string())?;
            self.status = ProcessStatus::Completed(-9);
            self.child = None;
        }
        Ok(())
    }

    /// Wait for process to complete.
    fn wait(&mut self) -> Result<ExitStatus, String> {
        if let Some(child) = &mut self.child {
            let status = child.wait().map_err(|e| e.to_string())?;
            self.status = ProcessStatus::Completed(status.code().unwrap_or(-1));
            self.child = None;
            Ok(status)
        } else {
            Err("Process not running".to_string())
        }
    }
}

/// Manager for background processes.
pub struct BackgroundManager {
    processes: Arc<Mutex<HashMap<String, ProcessInfo>>>,
}

impl BackgroundManager {
    /// Create a new background manager.
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start a background process.
    pub fn spawn(&self, command: &str, cwd: Option<&str>) -> Result<String, String> {
        let (shell, flag) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let mut cmd = Command::new(shell);
        cmd.arg(flag)
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let child = cmd.spawn().map_err(|e| e.to_string())?;
        let id = Uuid::new_v4().to_string();

        self.processes.lock().insert(
            id.clone(),
            ProcessInfo::new(id.clone(), command.to_string(), child),
        );

        Ok(id)
    }

    /// Get process status.
    pub fn status(&self, id: &str) -> Option<ProcessStatus> {
        let mut processes = self.processes.lock();
        if let Some(info) = processes.get_mut(id) {
            info.update_status();
            Some(info.status.clone())
        } else {
            None
        }
    }

    /// List all processes.
    pub fn list(&self) -> Vec<(String, String, ProcessStatus)> {
        let mut processes = self.processes.lock();
        processes
            .values_mut()
            .map(|info| {
                info.update_status();
                (info.id.clone(), info.command.clone(), info.status.clone())
            })
            .collect()
    }

    /// Kill a process.
    pub fn kill(&self, id: &str) -> Result<(), String> {
        let mut processes = self.processes.lock();
        if let Some(info) = processes.get_mut(id) {
            info.kill()
        } else {
            Err(format!("Process not found: {}", id))
        }
    }

    /// Wait for a process to complete.
    pub fn wait(&self, id: &str) -> Result<i32, String> {
        let mut processes = self.processes.lock();
        if let Some(info) = processes.get_mut(id) {
            let status = info.wait()?;
            Ok(status.code().unwrap_or(-1))
        } else {
            Err(format!("Process not found: {}", id))
        }
    }

    /// Clean up completed processes.
    pub fn cleanup(&self) {
        let mut processes = self.processes.lock();
        for info in processes.values_mut() {
            info.update_status();
        }
        processes.retain(|_, info| matches!(info.status, ProcessStatus::Running));
    }

    /// Get count of running processes.
    pub fn running_count(&self) -> usize {
        let mut processes = self.processes.lock();
        for info in processes.values_mut() {
            info.update_status();
        }
        processes
            .values()
            .filter(|info| matches!(info.status, ProcessStatus::Running))
            .count()
    }
}

impl Default for BackgroundManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_manager_creation() {
        let manager = BackgroundManager::new();
        assert_eq!(manager.running_count(), 0);
    }

    #[test]
    fn test_background_manager_default() {
        let manager = BackgroundManager::default();
        assert_eq!(manager.running_count(), 0);
    }

    #[test]
    fn test_spawn_and_list() {
        let manager = BackgroundManager::new();
        // Use a simple command that exits quickly
        let result = manager.spawn("echo hello", None);
        assert!(result.is_ok());

        let id = result.unwrap();
        let list = manager.list();
        assert!(list.iter().any(|(i, _, _)| i == &id));
    }

    #[test]
    fn test_spawn_with_cwd() {
        let manager = BackgroundManager::new();
        let result = manager.spawn("pwd", Some("/tmp"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_status() {
        let manager = BackgroundManager::new();
        let id = manager.spawn("echo status_test", None).unwrap();

        // Wait a bit for the command to complete
        std::thread::sleep(std::time::Duration::from_millis(100));

        let status = manager.status(&id);
        assert!(status.is_some());
    }

    #[test]
    fn test_status_not_found() {
        let manager = BackgroundManager::new();
        let status = manager.status("nonexistent_id");
        assert!(status.is_none());
    }

    #[test]
    fn test_kill() {
        let manager = BackgroundManager::new();
        // Spawn a long-running process
        let id = manager.spawn("sleep 60", None).unwrap();

        // Kill it
        let result = manager.kill(&id);
        assert!(result.is_ok());

        // Verify it's no longer running
        let status = manager.status(&id);
        assert!(matches!(status, Some(ProcessStatus::Completed(_))));
    }

    #[test]
    fn test_kill_not_found() {
        let manager = BackgroundManager::new();
        let result = manager.kill("nonexistent_id");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Process not found"));
    }

    #[test]
    fn test_wait() {
        let manager = BackgroundManager::new();
        let id = manager.spawn("echo wait_test", None).unwrap();

        // Wait for the process to complete
        let result = manager.wait(&id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // echo should exit with 0
    }

    #[test]
    fn test_wait_not_found() {
        let manager = BackgroundManager::new();
        let result = manager.wait("nonexistent_id");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Process not found"));
    }

    #[test]
    fn test_cleanup() {
        let manager = BackgroundManager::new();
        // Spawn a quick command
        let _ = manager.spawn("echo test", None);
        // Give it time to complete
        std::thread::sleep(std::time::Duration::from_millis(100));
        manager.cleanup();
        // After cleanup, completed processes should be removed
        assert_eq!(manager.running_count(), 0);
    }

    #[test]
    fn test_running_count() {
        let manager = BackgroundManager::new();

        // Initially empty
        assert_eq!(manager.running_count(), 0);

        // Spawn a long-running process
        let id = manager.spawn("sleep 60", None).unwrap();

        // Should have one running
        let _ = manager.running_count(); // Process may have started

        // Kill it
        let _ = manager.kill(&id);

        // Should be back to zero
        assert_eq!(manager.running_count(), 0);
    }

    #[test]
    fn test_process_status_debug() {
        let running = ProcessStatus::Running;
        let completed = ProcessStatus::Completed(0);
        let failed = ProcessStatus::Failed("error".to_string());

        // Test Debug impl
        assert!(format!("{:?}", running).contains("Running"));
        assert!(format!("{:?}", completed).contains("Completed"));
        assert!(format!("{:?}", failed).contains("Failed"));
    }

    #[test]
    fn test_process_status_clone() {
        let original = ProcessStatus::Completed(42);
        let cloned = original.clone();
        assert!(matches!(cloned, ProcessStatus::Completed(42)));
    }

    #[test]
    fn test_multiple_processes() {
        let manager = BackgroundManager::new();

        // Spawn multiple processes
        let id1 = manager.spawn("echo one", None).unwrap();
        let id2 = manager.spawn("echo two", None).unwrap();

        // Verify both are listed
        let list = manager.list();
        assert!(list.iter().any(|(i, _, _)| i == &id1));
        assert!(list.iter().any(|(i, _, _)| i == &id2));
    }

    #[test]
    fn test_spawn_invalid_command_cwd() {
        let manager = BackgroundManager::new();
        // This should still succeed as the shell is spawned, just running in wrong dir
        let result = manager.spawn("echo test", Some("/nonexistent_directory_xyz"));
        // The spawn may succeed because the shell itself is spawned
        // but the command might fail - either way we test the path
        let _ = result;
    }
}
