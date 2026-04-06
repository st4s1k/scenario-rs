use serde::{Deserialize, Serialize};

/// Top-level execution status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Idle,
    Running,
    Completed,
    Failed { error: String },
}

/// Status of an individual step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Progress information for a currently executing task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskProgress {
    SftpCopy {
        source: String,
        destination: String,
        bytes_transferred: u64,
        bytes_total: u64,
    },
    RemoteSudo {
        command: String,
        output: String,
    },
}

/// Execution state for a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecState {
    pub index: usize,
    pub task_description: String,
    pub status: StepStatus,
    pub progress: Option<TaskProgress>,
    pub output: String,
    pub errors: Vec<String>,
    pub on_fail_steps: Vec<OnFailStepExecState>,
}

/// Execution state for a single on-fail recovery step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnFailStepExecState {
    pub index: usize,
    pub task_description: String,
    pub status: StepStatus,
    pub progress: Option<TaskProgress>,
    pub output: String,
    pub errors: Vec<String>,
}

/// Full execution state snapshot, queryable at any time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub status: ExecutionStatus,
    pub steps: Vec<StepExecState>,
}

/// A granular change to the execution state, streamed to subscribers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StateDiff {
    ExecutionStatusChanged {
        status: ExecutionStatus,
    },
    StepStatusChanged {
        step_index: usize,
        status: StepStatus,
    },
    StepProgressUpdated {
        step_index: usize,
        progress: TaskProgress,
    },
    StepOutputAppended {
        step_index: usize,
        text: String,
    },
    StepErrorAdded {
        step_index: usize,
        error: String,
    },
    OnFailStepStatusChanged {
        step_index: usize,
        on_fail_step_index: usize,
        status: StepStatus,
    },
    OnFailStepProgressUpdated {
        step_index: usize,
        on_fail_step_index: usize,
        progress: TaskProgress,
    },
    OnFailStepOutputAppended {
        step_index: usize,
        on_fail_step_index: usize,
        text: String,
    },
    OnFailStepErrorAdded {
        step_index: usize,
        on_fail_step_index: usize,
        error: String,
    },
}
