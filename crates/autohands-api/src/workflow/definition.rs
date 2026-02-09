//! Workflow definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Workflow step type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// Execute a single agent.
    Agent { agent: String, prompt: String },
    /// Execute steps in parallel.
    Parallel { steps: Vec<WorkflowStep> },
    /// Execute steps in sequence.
    Sequential { steps: Vec<WorkflowStep> },
    /// Conditional branch.
    Conditional {
        condition: String,
        if_true: Box<WorkflowStep>,
        if_false: Option<Box<WorkflowStep>>,
    },
    /// Wait for an event.
    WaitForEvent {
        event_type: String,
        timeout_secs: Option<u64>,
    },
}

/// A workflow step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step ID.
    pub id: String,
    /// Step name.
    pub name: String,
    /// Step type.
    pub step_type: StepType,
    /// Timeout in seconds.
    pub timeout_secs: Option<u64>,
}

impl WorkflowStep {
    /// Create a new agent step.
    pub fn agent(
        id: impl Into<String>,
        name: impl Into<String>,
        agent: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            step_type: StepType::Agent {
                agent: agent.into(),
                prompt: prompt.into(),
            },
            timeout_secs: None,
        }
    }

    /// Create a parallel step.
    pub fn parallel(
        id: impl Into<String>,
        name: impl Into<String>,
        steps: Vec<WorkflowStep>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            step_type: StepType::Parallel { steps },
            timeout_secs: None,
        }
    }

    /// Create a sequential step.
    pub fn sequential(
        id: impl Into<String>,
        name: impl Into<String>,
        steps: Vec<WorkflowStep>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            step_type: StepType::Sequential { steps },
            timeout_secs: None,
        }
    }

    /// Create a conditional step.
    pub fn conditional(
        id: impl Into<String>,
        name: impl Into<String>,
        condition: impl Into<String>,
        if_true: WorkflowStep,
        if_false: Option<WorkflowStep>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            step_type: StepType::Conditional {
                condition: condition.into(),
                if_true: Box::new(if_true),
                if_false: if_false.map(Box::new),
            },
            timeout_secs: None,
        }
    }

    /// Set timeout.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }
}

/// A workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow ID.
    pub id: String,
    /// Workflow name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Root step.
    pub root: WorkflowStep,
    /// Timeout for entire workflow.
    pub timeout_secs: Option<u64>,
}

impl Workflow {
    /// Create a new workflow.
    pub fn new(id: impl Into<String>, name: impl Into<String>, root: WorkflowStep) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            root,
            timeout_secs: None,
        }
    }

    /// Set description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }
}

/// Workflow execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionState {
    /// Not started.
    Pending,
    /// Currently running.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed.
    Failed,
    /// Cancelled.
    Cancelled,
}

/// Workflow execution instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    /// Execution ID.
    pub id: Uuid,
    /// Workflow ID.
    pub workflow_id: String,
    /// Current state.
    pub state: ExecutionState,
    /// Start time.
    pub started_at: DateTime<Utc>,
    /// End time.
    pub ended_at: Option<DateTime<Utc>>,
    /// Current step ID.
    pub current_step: Option<String>,
    /// Step results.
    pub step_results: serde_json::Value,
    /// Error message if failed.
    pub error: Option<String>,
}

impl WorkflowExecution {
    /// Create a new execution.
    pub fn new(workflow_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            workflow_id: workflow_id.into(),
            state: ExecutionState::Pending,
            started_at: Utc::now(),
            ended_at: None,
            current_step: None,
            step_results: serde_json::json!({}),
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
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
}
