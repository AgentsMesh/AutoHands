
    use super::*;

    #[test]
    fn test_workflow_step_agent() {
        let step = WorkflowStep::agent("step1", "First Step", "general", "Do something");
        assert_eq!(step.id, "step1");
        assert!(matches!(step.step_type, StepType::Agent { .. }));
    }

    #[test]
    fn test_workflow_step_parallel() {
        let steps = vec![
            WorkflowStep::agent("s1", "Step 1", "a1", "p1"),
            WorkflowStep::agent("s2", "Step 2", "a2", "p2"),
        ];
        let parallel = WorkflowStep::parallel("par", "Parallel Steps", steps);
        assert!(matches!(parallel.step_type, StepType::Parallel { .. }));
    }

    #[test]
    fn test_workflow_new() {
        let root = WorkflowStep::agent("root", "Root", "general", "Start");
        let workflow = Workflow::new("wf1", "Test Workflow", root);
        assert_eq!(workflow.id, "wf1");
    }

    #[test]
    fn test_execution_new() {
        let exec = WorkflowExecution::new("wf1");
        assert_eq!(exec.workflow_id, "wf1");
        assert_eq!(exec.state, ExecutionState::Pending);
    }
