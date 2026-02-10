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
#[path = "background_tests.rs"]
mod tests;
