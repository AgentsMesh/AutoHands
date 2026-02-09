//! RunLoop mode definitions.
//!
//! Modes provide isolation for different types of event processing,
//! inspired by iOS CFRunLoopMode design.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// RunLoop running mode.
///
/// Similar to CFRunLoopMode in iOS, modes provide isolation
/// for different types of event processing.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum RunLoopMode {
    /// Default mode - processes all regular events.
    /// Similar to kCFRunLoopDefaultMode.
    Default,

    /// Agent processing mode - focuses on Agent execution,
    /// suspends low-priority events.
    /// Similar to UITrackingRunLoopMode (ensures smooth UI scrolling).
    AgentProcessing,

    /// Background mode - only processes low-priority maintenance tasks.
    Background,

    /// Common modes set - includes Default + AgentProcessing.
    /// Sources added to Common are automatically synced to these modes.
    /// Similar to kCFRunLoopCommonModes.
    Common,

    /// Custom mode for specialized use cases.
    Custom(String),
}

impl Default for RunLoopMode {
    fn default() -> Self {
        RunLoopMode::Default
    }
}

impl std::fmt::Display for RunLoopMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunLoopMode::Default => write!(f, "default"),
            RunLoopMode::AgentProcessing => write!(f, "agent_processing"),
            RunLoopMode::Background => write!(f, "background"),
            RunLoopMode::Common => write!(f, "common"),
            RunLoopMode::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

impl RunLoopMode {
    /// Check if this mode is included in the Common modes set.
    pub fn is_common_mode(&self) -> bool {
        matches!(self, RunLoopMode::Default | RunLoopMode::AgentProcessing)
    }

    /// Get the default common modes set.
    pub fn default_common_modes() -> HashSet<RunLoopMode> {
        let mut modes = HashSet::new();
        modes.insert(RunLoopMode::Default);
        modes.insert(RunLoopMode::AgentProcessing);
        modes
    }
}

/// RunLoop execution phase.
///
/// Corresponds to CFRunLoopActivity in iOS.
/// Observers can watch for these phases to perform work at specific points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum RunLoopPhase {
    /// Entering the RunLoop (kCFRunLoopEntry).
    Entry = 1 << 0,

    /// About to process timers (kCFRunLoopBeforeTimers).
    BeforeTimers = 1 << 1,

    /// About to process sources (kCFRunLoopBeforeSources).
    BeforeSources = 1 << 2,

    /// About to sleep/wait (kCFRunLoopBeforeWaiting).
    /// Key phase: batch commit events, create checkpoints, release resources.
    BeforeWaiting = 1 << 5,

    /// Just woke up from sleep (kCFRunLoopAfterWaiting).
    AfterWaiting = 1 << 6,

    /// Exiting the RunLoop (kCFRunLoopExit).
    Exit = 1 << 7,
}

impl RunLoopPhase {
    /// Get all phases as a bitmask.
    pub const ALL: u32 = Self::Entry as u32
        | Self::BeforeTimers as u32
        | Self::BeforeSources as u32
        | Self::BeforeWaiting as u32
        | Self::AfterWaiting as u32
        | Self::Exit as u32;

    /// Check if this phase is included in the given activity mask.
    pub fn matches(&self, activities: u32) -> bool {
        (activities & (*self as u32)) != 0
    }
}

/// RunLoop state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RunLoopState {
    /// Initial state, not started.
    Created = 0,
    /// Running and processing events.
    Running = 1,
    /// Waiting/sleeping for events.
    Waiting = 2,
    /// Stopping.
    Stopping = 3,
    /// Stopped.
    Stopped = 4,
}

impl From<u8> for RunLoopState {
    fn from(v: u8) -> Self {
        match v {
            0 => RunLoopState::Created,
            1 => RunLoopState::Running,
            2 => RunLoopState::Waiting,
            3 => RunLoopState::Stopping,
            4 => RunLoopState::Stopped,
            _ => RunLoopState::Created,
        }
    }
}

impl std::fmt::Display for RunLoopState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunLoopState::Created => write!(f, "created"),
            RunLoopState::Running => write!(f, "running"),
            RunLoopState::Waiting => write!(f, "waiting"),
            RunLoopState::Stopping => write!(f, "stopping"),
            RunLoopState::Stopped => write!(f, "stopped"),
        }
    }
}

/// RunLoop run result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunLoopRunResult {
    /// RunLoop finished normally.
    Finished,
    /// RunLoop was stopped.
    Stopped,
    /// RunLoop timed out.
    TimedOut,
    /// Source or timer handled event.
    HandledSource,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_display() {
        assert_eq!(RunLoopMode::Default.to_string(), "default");
        assert_eq!(RunLoopMode::AgentProcessing.to_string(), "agent_processing");
        assert_eq!(
            RunLoopMode::Custom("test".to_string()).to_string(),
            "custom:test"
        );
    }

    #[test]
    fn test_mode_is_common() {
        assert!(RunLoopMode::Default.is_common_mode());
        assert!(RunLoopMode::AgentProcessing.is_common_mode());
        assert!(!RunLoopMode::Background.is_common_mode());
        assert!(!RunLoopMode::Common.is_common_mode());
    }

    #[test]
    fn test_default_common_modes() {
        let modes = RunLoopMode::default_common_modes();
        assert!(modes.contains(&RunLoopMode::Default));
        assert!(modes.contains(&RunLoopMode::AgentProcessing));
        assert!(!modes.contains(&RunLoopMode::Background));
    }

    #[test]
    fn test_phase_matches() {
        let activities = RunLoopPhase::Entry as u32 | RunLoopPhase::Exit as u32;
        assert!(RunLoopPhase::Entry.matches(activities));
        assert!(RunLoopPhase::Exit.matches(activities));
        assert!(!RunLoopPhase::BeforeWaiting.matches(activities));
    }

    #[test]
    fn test_phase_all() {
        assert!(RunLoopPhase::Entry.matches(RunLoopPhase::ALL));
        assert!(RunLoopPhase::BeforeTimers.matches(RunLoopPhase::ALL));
        assert!(RunLoopPhase::BeforeSources.matches(RunLoopPhase::ALL));
        assert!(RunLoopPhase::BeforeWaiting.matches(RunLoopPhase::ALL));
        assert!(RunLoopPhase::AfterWaiting.matches(RunLoopPhase::ALL));
        assert!(RunLoopPhase::Exit.matches(RunLoopPhase::ALL));
    }

    #[test]
    fn test_state_from_u8() {
        assert_eq!(RunLoopState::from(0), RunLoopState::Created);
        assert_eq!(RunLoopState::from(1), RunLoopState::Running);
        assert_eq!(RunLoopState::from(2), RunLoopState::Waiting);
        assert_eq!(RunLoopState::from(3), RunLoopState::Stopping);
        assert_eq!(RunLoopState::from(4), RunLoopState::Stopped);
        assert_eq!(RunLoopState::from(99), RunLoopState::Created);
    }

    #[test]
    fn test_state_display() {
        assert_eq!(RunLoopState::Running.to_string(), "running");
        assert_eq!(RunLoopState::Waiting.to_string(), "waiting");
    }

    #[test]
    fn test_mode_default() {
        let mode: RunLoopMode = Default::default();
        assert_eq!(mode, RunLoopMode::Default);
    }

    #[test]
    fn test_mode_eq() {
        assert_eq!(RunLoopMode::Default, RunLoopMode::Default);
        assert_ne!(RunLoopMode::Default, RunLoopMode::Background);
        assert_eq!(
            RunLoopMode::Custom("a".to_string()),
            RunLoopMode::Custom("a".to_string())
        );
        assert_ne!(
            RunLoopMode::Custom("a".to_string()),
            RunLoopMode::Custom("b".to_string())
        );
    }
}
