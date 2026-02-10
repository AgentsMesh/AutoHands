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
