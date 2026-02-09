//! File watcher for skill hot-reload.
//!
//! Monitors skill directories for changes and triggers reload.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::Skill;

use super::filesystem::FilesystemLoader;

/// File watcher for skill hot-reload.
pub struct SkillWatcher {
    /// Shared skill storage.
    skills: Arc<RwLock<HashMap<String, Skill>>>,
    /// Filesystem loader for reloading skills.
    fs_loader: FilesystemLoader,
    /// Available tools for eligibility checking.
    available_tools: Arc<RwLock<Vec<String>>>,
    /// Watched paths.
    watched_paths: Vec<PathBuf>,
    /// Internal watcher handle.
    _watcher: Option<RecommendedWatcher>,
    /// Shutdown sender.
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl SkillWatcher {
    /// Create a new skill watcher.
    pub fn new(
        skills: Arc<RwLock<HashMap<String, Skill>>>,
        fs_loader: FilesystemLoader,
        available_tools: Arc<RwLock<Vec<String>>>,
    ) -> Self {
        Self {
            skills,
            fs_loader,
            available_tools,
            watched_paths: Vec::new(),
            _watcher: None,
            shutdown_tx: None,
        }
    }

    /// Watch a directory for changes.
    pub fn watch(&mut self, path: PathBuf) -> Result<(), SkillError> {
        if !path.exists() {
            debug!("Watch path does not exist, skipping: {}", path.display());
            return Ok(());
        }

        self.watched_paths.push(path);
        Ok(())
    }

    /// Start watching all registered paths.
    pub fn start(&mut self) -> Result<(), SkillError> {
        if self.watched_paths.is_empty() {
            return Ok(());
        }

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(100);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Create the notify watcher
        let watcher_tx = event_tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let _ = watcher_tx.blocking_send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )
        .map_err(|e| SkillError::LoadingFailed(format!("Failed to create watcher: {}", e)))?;

        // Watch all paths
        for path in &self.watched_paths {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| {
                    SkillError::LoadingFailed(format!(
                        "Failed to watch {}: {}",
                        path.display(),
                        e
                    ))
                })?;
            info!("Watching for skill changes: {}", path.display());
        }

        self._watcher = Some(watcher);
        self.shutdown_tx = Some(shutdown_tx);

        // Clone what we need for the async task
        let skills = self.skills.clone();
        let fs_loader = self.fs_loader.clone();
        let available_tools = self.available_tools.clone();
        let watched_paths = self.watched_paths.clone();

        // Spawn the event handling task
        tokio::spawn(async move {
            let mut debounce_timer: Option<tokio::time::Instant> = None;
            let debounce_duration = Duration::from_millis(500);

            loop {
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        if Self::is_relevant_event(&event) {
                            debug!("Skill file change detected: {:?}", event.paths);
                            debounce_timer = Some(tokio::time::Instant::now());
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Skill watcher shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        if let Some(timer) = debounce_timer {
                            if timer.elapsed() >= debounce_duration {
                                debounce_timer = None;
                                if let Err(e) = Self::reload_skills(
                                    &skills,
                                    &fs_loader,
                                    &available_tools,
                                    &watched_paths,
                                ).await {
                                    error!("Failed to reload skills: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Check if an event is relevant for skill reloading.
    fn is_relevant_event(event: &Event) -> bool {
        // Only care about create, modify, and remove events
        matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        ) && event.paths.iter().any(|p| {
            // Check if it's a skill file
            if let Some(ext) = p.extension() {
                let ext = ext.to_str().unwrap_or("");
                if ext == "markdown" || ext == "md" {
                    return true;
                }
            }
            // Or a SKILL.* file
            if let Some(name) = p.file_stem() {
                if name.to_str().unwrap_or("") == "SKILL" {
                    return true;
                }
            }
            false
        })
    }

    /// Reload all skills from watched paths.
    async fn reload_skills(
        skills: &Arc<RwLock<HashMap<String, Skill>>>,
        fs_loader: &FilesystemLoader,
        available_tools: &Arc<RwLock<Vec<String>>>,
        watched_paths: &[PathBuf],
    ) -> Result<(), SkillError> {
        info!("Reloading skills...");

        let mut all_skills = HashMap::new();
        let tools = available_tools.read().await;

        for path in watched_paths {
            if path.exists() {
                let loaded = fs_loader.load_from_directory(path).await?;
                for skill in loaded {
                    // Check tool dependencies
                    let eligible = skill.definition.required_tools.iter().all(|t| tools.contains(t));
                    if eligible {
                        all_skills.insert(skill.definition.id.clone(), skill);
                    } else {
                        warn!(
                            "Skill {} not eligible (missing tools)",
                            skill.definition.id
                        );
                    }
                }
            }
        }

        let mut skills_guard = skills.write().await;
        let old_count = skills_guard.len();
        *skills_guard = all_skills;
        let new_count = skills_guard.len();

        info!(
            "Skills reloaded: {} -> {} skills",
            old_count, new_count
        );

        Ok(())
    }

    /// Stop watching.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.blocking_send(());
        }
        self._watcher = None;
    }
}

impl Drop for SkillWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skill(path: &PathBuf) {
        let content = r#"---
id: test-watch
name: Watch Test
description: Test skill for watching
---

Test content.
"#;
        fs::write(path, content).unwrap();
    }

    #[tokio::test]
    async fn test_watcher_creation() {
        let skills = Arc::new(RwLock::new(HashMap::new()));
        let fs_loader = FilesystemLoader::new();
        let available_tools = Arc::new(RwLock::new(Vec::new()));

        let watcher = SkillWatcher::new(skills, fs_loader, available_tools);
        assert!(watcher.watched_paths.is_empty());
    }

    #[tokio::test]
    async fn test_watch_path() {
        let temp_dir = TempDir::new().unwrap();
        let skills = Arc::new(RwLock::new(HashMap::new()));
        let fs_loader = FilesystemLoader::new();
        let available_tools = Arc::new(RwLock::new(Vec::new()));

        let mut watcher = SkillWatcher::new(skills, fs_loader, available_tools);
        watcher.watch(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(watcher.watched_paths.len(), 1);
    }

    #[tokio::test]
    async fn test_watch_nonexistent_path() {
        let skills = Arc::new(RwLock::new(HashMap::new()));
        let fs_loader = FilesystemLoader::new();
        let available_tools = Arc::new(RwLock::new(Vec::new()));

        let mut watcher = SkillWatcher::new(skills, fs_loader, available_tools);
        let result = watcher.watch(PathBuf::from("/nonexistent/path"));

        // Should not error, just skip
        assert!(result.is_ok());
        assert!(watcher.watched_paths.is_empty());
    }

    #[test]
    fn test_is_relevant_event() {
        use notify::event::{CreateKind, ModifyKind, RemoveKind};

        // Create event for markdown file - should be relevant
        let create_event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![PathBuf::from("/test/skill.markdown")],
            attrs: Default::default(),
        };
        assert!(SkillWatcher::is_relevant_event(&create_event));

        // Modify event for SKILL.md - should be relevant
        let modify_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![PathBuf::from("/test/my-skill/SKILL.md")],
            attrs: Default::default(),
        };
        assert!(SkillWatcher::is_relevant_event(&modify_event));

        // Remove event - should be relevant
        let remove_event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![PathBuf::from("/test/old.markdown")],
            attrs: Default::default(),
        };
        assert!(SkillWatcher::is_relevant_event(&remove_event));

        // Non-skill file - should not be relevant
        let other_event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![PathBuf::from("/test/file.txt")],
            attrs: Default::default(),
        };
        assert!(!SkillWatcher::is_relevant_event(&other_event));
    }

    #[tokio::test]
    async fn test_reload_skills() {
        let temp_dir = TempDir::new().unwrap();
        let skill_path = temp_dir.path().join("reload-test.markdown");
        create_test_skill(&skill_path);

        let skills = Arc::new(RwLock::new(HashMap::new()));
        let fs_loader = FilesystemLoader::new();
        let available_tools = Arc::new(RwLock::new(Vec::new()));
        let watched_paths = vec![temp_dir.path().to_path_buf()];

        SkillWatcher::reload_skills(&skills, &fs_loader, &available_tools, &watched_paths)
            .await
            .unwrap();

        let skills_guard = skills.read().await;
        assert_eq!(skills_guard.len(), 1);
        assert!(skills_guard.contains_key("test-watch"));
    }
}
