//! PID file management for daemon processes.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};

use crate::error::DaemonError;

/// PID file manager for preventing duplicate daemon instances.
#[derive(Debug)]
pub struct PidFile {
    path: PathBuf,
    locked: bool,
}

impl PidFile {
    /// Create a new PID file manager.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            locked: false,
        }
    }

    /// Get the PID file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if a PID file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Read the PID from the file.
    pub fn read_pid(&self) -> Result<Option<u32>, DaemonError> {
        if !self.exists() {
            return Ok(None);
        }

        let mut file = File::open(&self.path).map_err(|e| DaemonError::PidFileRead {
            path: self.path.clone(),
            reason: e.to_string(),
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| DaemonError::PidFileRead {
                path: self.path.clone(),
                reason: e.to_string(),
            })?;

        let pid = contents
            .trim()
            .parse::<u32>()
            .map_err(|e| DaemonError::PidFileRead {
                path: self.path.clone(),
                reason: format!("Invalid PID format: {}", e),
            })?;

        Ok(Some(pid))
    }

    /// Write the current process PID to the file.
    pub fn write_pid(&mut self) -> Result<(), DaemonError> {
        let pid = std::process::id();
        self.write_pid_value(pid)
    }

    /// Write a specific PID value to the file.
    pub fn write_pid_value(&mut self, pid: u32) -> Result<(), DaemonError> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| DaemonError::PidFileCreation {
                path: self.path.clone(),
                reason: format!("Failed to create parent directory: {}", e),
            })?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| DaemonError::PidFileCreation {
                path: self.path.clone(),
                reason: e.to_string(),
            })?;

        write!(file, "{}", pid).map_err(|e| DaemonError::PidFileCreation {
            path: self.path.clone(),
            reason: e.to_string(),
        })?;

        self.locked = true;
        info!("PID file created: {} (PID: {})", self.path.display(), pid);
        Ok(())
    }

    /// Remove the PID file.
    pub fn remove(&mut self) -> Result<(), DaemonError> {
        if !self.exists() {
            self.locked = false;
            return Ok(());
        }

        fs::remove_file(&self.path).map_err(|e| DaemonError::PidFileRemoval {
            path: self.path.clone(),
            reason: e.to_string(),
        })?;

        self.locked = false;
        info!("PID file removed: {}", self.path.display());
        Ok(())
    }

    /// Check if a process with the given PID is running.
    #[cfg(unix)]
    pub fn is_process_running(pid: u32) -> bool {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        // Send signal 0 to check if process exists
        kill(Pid::from_raw(pid as i32), Some(Signal::SIGCONT))
            .map(|_| true)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    pub fn is_process_running(_pid: u32) -> bool {
        // On non-Unix systems, assume process is running if we can't check
        true
    }

    /// Try to acquire the PID file lock.
    /// Returns Ok if successful, error if daemon is already running.
    pub fn try_acquire(&mut self) -> Result<(), DaemonError> {
        if let Some(existing_pid) = self.read_pid()? {
            if Self::is_process_running(existing_pid) {
                return Err(DaemonError::AlreadyRunning {
                    path: self.path.clone(),
                    pid: existing_pid,
                });
            }

            // Stale PID file - remove it
            warn!(
                "Removing stale PID file (PID {} not running): {}",
                existing_pid,
                self.path.display()
            );
            self.remove()?;
        }

        self.write_pid()
    }

    /// Check if we hold the lock.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Force remove the PID file (used for cleanup).
    pub fn force_remove(&mut self) -> Result<(), DaemonError> {
        if self.exists() {
            debug!("Force removing PID file: {}", self.path.display());
            self.remove()
        } else {
            Ok(())
        }
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        if self.locked {
            if let Err(e) = self.remove() {
                warn!("Failed to remove PID file on drop: {}", e);
            }
        }
    }
}

#[cfg(test)]
#[path = "pid_tests.rs"]
mod tests;
