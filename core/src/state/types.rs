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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_execution_state() -> ExecutionState {
        ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![StepExecState {
                index: 0,
                task_description: "deploy".into(),
                status: StepStatus::Completed,
                progress: Some(TaskProgress::SftpCopy {
                    source: "/tmp/a".into(),
                    destination: "/opt/b".into(),
                    bytes_transferred: 50,
                    bytes_total: 100,
                }),
                output: "done\n".into(),
                errors: vec!["warning".into()],
                on_fail_steps: vec![OnFailStepExecState {
                    index: 0,
                    task_description: "rollback".into(),
                    status: StepStatus::Pending,
                    progress: None,
                    output: String::new(),
                    errors: Vec::new(),
                }],
            }],
        }
    }

    #[test]
    fn execution_state_round_trip() {
        // Given
        let state = sample_execution_state();

        // When
        let json = serde_json::to_string(&state).unwrap();
        let restored: ExecutionState = serde_json::from_str(&json).unwrap();

        // Then
        assert_eq!(restored.status, ExecutionStatus::Running);
        assert_eq!(restored.steps.len(), 1);
        assert_eq!(restored.steps[0].status, StepStatus::Completed);
        assert_eq!(restored.steps[0].output, "done\n");
        assert_eq!(restored.steps[0].errors, vec!["warning"]);
        assert_eq!(restored.steps[0].on_fail_steps.len(), 1);
        assert_eq!(restored.steps[0].on_fail_steps[0].status, StepStatus::Pending);
    }

    #[test]
    fn task_progress_serde_has_type_tag() {
        // Given & When & Then
        let p = TaskProgress::SftpCopy {
            source: "a".into(),
            destination: "b".into(),
            bytes_transferred: 10,
            bytes_total: 20,
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["type"], "SftpCopy");

        let p2 = TaskProgress::RemoteSudo {
            command: "ls".into(),
            output: "file.txt".into(),
        };
        let json2 = serde_json::to_value(&p2).unwrap();
        assert_eq!(json2["type"], "RemoteSudo");
    }

    #[test]
    fn state_diff_serde_has_kind_tag() {
        // Given
        let cases = vec![
            (StateDiff::ExecutionStatusChanged { status: ExecutionStatus::Completed }, "ExecutionStatusChanged"),
            (StateDiff::StepStatusChanged { step_index: 0, status: StepStatus::Running }, "StepStatusChanged"),
            (StateDiff::StepOutputAppended { step_index: 1, text: "hello".into() }, "StepOutputAppended"),
            (StateDiff::StepErrorAdded { step_index: 0, error: "boom".into() }, "StepErrorAdded"),
            (StateDiff::OnFailStepStatusChanged { step_index: 0, on_fail_step_index: 1, status: StepStatus::Failed }, "OnFailStepStatusChanged"),
            (StateDiff::OnFailStepOutputAppended { step_index: 0, on_fail_step_index: 0, text: "out".into() }, "OnFailStepOutputAppended"),
            (StateDiff::OnFailStepErrorAdded { step_index: 0, on_fail_step_index: 0, error: "err".into() }, "OnFailStepErrorAdded"),
        ];

        for (diff, expected_kind) in cases {
            // When & Then
            let json = serde_json::to_value(&diff).unwrap();
            assert_eq!(json["kind"], expected_kind, "wrong kind tag for {:?}", diff);
            let restored: StateDiff = serde_json::from_value(json).unwrap();
            let json2 = serde_json::to_value(&restored).unwrap();
            assert_eq!(json2["kind"], expected_kind);
        }
    }

    #[test]
    fn step_progress_diff_round_trip() {
        // Given
        let diff = StateDiff::StepProgressUpdated {
            step_index: 2,
            progress: TaskProgress::SftpCopy {
                source: "src".into(),
                destination: "dst".into(),
                bytes_transferred: 42,
                bytes_total: 100,
            },
        };

        // When
        let json = serde_json::to_string(&diff).unwrap();
        let restored: StateDiff = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_value(&restored).unwrap();

        // Then
        assert_eq!(json2["kind"], "StepProgressUpdated");
        assert_eq!(json2["step_index"], 2);
        assert_eq!(json2["progress"]["type"], "SftpCopy");
        assert_eq!(json2["progress"]["bytes_transferred"], 42);
    }

    #[test]
    fn on_fail_progress_diff_round_trip() {
        // Given
        let diff = StateDiff::OnFailStepProgressUpdated {
            step_index: 0,
            on_fail_step_index: 3,
            progress: TaskProgress::RemoteSudo {
                command: "restart".into(),
                output: "ok".into(),
            },
        };

        // When
        let json = serde_json::to_string(&diff).unwrap();
        let restored: StateDiff = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_value(&restored).unwrap();

        // Then
        assert_eq!(json2["kind"], "OnFailStepProgressUpdated");
        assert_eq!(json2["on_fail_step_index"], 3);
        assert_eq!(json2["progress"]["command"], "restart");
    }

    #[test]
    fn execution_status_failed_serde() {
        // Given
        let status = ExecutionStatus::Failed { error: "connection lost".into() };

        // When
        let json = serde_json::to_value(&status).unwrap();
        let restored: ExecutionStatus = serde_json::from_value(json).unwrap();

        // Then
        assert_eq!(restored, ExecutionStatus::Failed { error: "connection lost".into() });
    }

    #[test]
    fn all_step_statuses_serde() {
        // Given
        let statuses = vec![
            StepStatus::Pending,
            StepStatus::Running,
            StepStatus::Completed,
            StepStatus::Failed,
            StepStatus::Skipped,
        ];

        for s in statuses {
            // When
            let json = serde_json::to_value(&s).unwrap();
            let restored: StepStatus = serde_json::from_value(json).unwrap();

            // Then
            assert_eq!(restored, s);
        }
    }
}
