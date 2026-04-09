use crate::state::types::*;
use std::sync::{mpsc::Sender, Arc, RwLock};

/// Thread-safe manager for execution state with diff streaming.
///
/// Maintains a canonical `ExecutionState` and emits `StateDiff` events
/// through an mpsc channel whenever the state changes.
pub struct ExecutionStateManager {
    state: Arc<RwLock<ExecutionState>>,
    diff_tx: Sender<StateDiff>,
}

impl ExecutionStateManager {
    /// Creates a new state manager with the given initial state and diff sender.
    pub fn new(initial_state: ExecutionState, diff_tx: Sender<StateDiff>) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial_state)),
            diff_tx,
        }
    }

    /// Returns a full clone of the current execution state.
    pub fn snapshot(&self) -> ExecutionState {
        self.state.read().unwrap().clone()
    }

    fn emit(&self, diff: StateDiff) {
        let _ = self.diff_tx.send(diff);
    }

    pub fn update_execution_status(&self, status: ExecutionStatus) {
        {
            let mut state = self.state.write().unwrap();
            state.status = status.clone();
        }
        self.emit(StateDiff::ExecutionStatusChanged { status });
    }

    pub fn update_step_status(&self, step_index: usize, status: StepStatus) {
        {
            let mut state = self.state.write().unwrap();
            if let Some(step) = state.steps.get_mut(step_index) {
                step.status = status.clone();
            }
        }
        self.emit(StateDiff::StepStatusChanged { step_index, status });
    }

    pub fn update_step_progress(&self, step_index: usize, progress: TaskProgress) {
        {
            let mut state = self.state.write().unwrap();
            if let Some(step) = state.steps.get_mut(step_index) {
                step.progress = Some(progress.clone());
            }
        }
        self.emit(StateDiff::StepProgressUpdated {
            step_index,
            progress,
        });
    }

    pub fn append_step_output(&self, step_index: usize, text: String) {
        {
            let mut state = self.state.write().unwrap();
            if let Some(step) = state.steps.get_mut(step_index) {
                if !step.output.is_empty() {
                    step.output.push('\n');
                }
                step.output.push_str(&text);
            }
        }
        self.emit(StateDiff::StepOutputAppended { step_index, text });
    }

    pub fn add_step_error(&self, step_index: usize, error: String) {
        {
            let mut state = self.state.write().unwrap();
            if let Some(step) = state.steps.get_mut(step_index) {
                step.errors.push(error.clone());
            }
        }
        self.emit(StateDiff::StepErrorAdded { step_index, error });
    }

    pub fn update_on_fail_step_status(
        &self,
        step_index: usize,
        on_fail_step_index: usize,
        status: StepStatus,
    ) {
        {
            let mut state = self.state.write().unwrap();
            if let Some(step) = state.steps.get_mut(step_index) {
                if let Some(ofs) = step.on_fail_steps.get_mut(on_fail_step_index) {
                    ofs.status = status.clone();
                }
            }
        }
        self.emit(StateDiff::OnFailStepStatusChanged {
            step_index,
            on_fail_step_index,
            status,
        });
    }

    pub fn update_on_fail_step_progress(
        &self,
        step_index: usize,
        on_fail_step_index: usize,
        progress: TaskProgress,
    ) {
        {
            let mut state = self.state.write().unwrap();
            if let Some(step) = state.steps.get_mut(step_index) {
                if let Some(ofs) = step.on_fail_steps.get_mut(on_fail_step_index) {
                    ofs.progress = Some(progress.clone());
                }
            }
        }
        self.emit(StateDiff::OnFailStepProgressUpdated {
            step_index,
            on_fail_step_index,
            progress,
        });
    }

    pub fn append_on_fail_step_output(
        &self,
        step_index: usize,
        on_fail_step_index: usize,
        text: String,
    ) {
        {
            let mut state = self.state.write().unwrap();
            if let Some(step) = state.steps.get_mut(step_index) {
                if let Some(ofs) = step.on_fail_steps.get_mut(on_fail_step_index) {
                    if !ofs.output.is_empty() {
                        ofs.output.push('\n');
                    }
                    ofs.output.push_str(&text);
                }
            }
        }
        self.emit(StateDiff::OnFailStepOutputAppended {
            step_index,
            on_fail_step_index,
            text,
        });
    }

    pub fn add_on_fail_step_error(
        &self,
        step_index: usize,
        on_fail_step_index: usize,
        error: String,
    ) {
        {
            let mut state = self.state.write().unwrap();
            if let Some(step) = state.steps.get_mut(step_index) {
                if let Some(ofs) = step.on_fail_steps.get_mut(on_fail_step_index) {
                    ofs.errors.push(error.clone());
                }
            }
        }
        self.emit(StateDiff::OnFailStepErrorAdded {
            step_index,
            on_fail_step_index,
            error,
        });
    }
}

/// A convenience handle for task-level progress reporting.
///
/// Created by step/on-fail-step execution code and passed to
/// `RemoteSudo::execute` / `SftpCopy::execute` so they can report
/// progress without knowing which step they belong to.
pub struct TaskTracker<'a> {
    manager: &'a ExecutionStateManager,
    step_index: usize,
    on_fail_step_index: Option<usize>,
}

impl<'a> TaskTracker<'a> {
    /// Creates a tracker for a regular step's task.
    pub fn for_step(manager: &'a ExecutionStateManager, step_index: usize) -> Self {
        Self {
            manager,
            step_index,
            on_fail_step_index: None,
        }
    }

    /// Creates a tracker for an on-fail step's task.
    pub fn for_on_fail_step(
        manager: &'a ExecutionStateManager,
        step_index: usize,
        on_fail_step_index: usize,
    ) -> Self {
        Self {
            manager,
            step_index,
            on_fail_step_index: Some(on_fail_step_index),
        }
    }

    pub fn update_progress(&self, progress: TaskProgress) {
        match self.on_fail_step_index {
            None => self.manager.update_step_progress(self.step_index, progress),
            Some(idx) => {
                self.manager
                    .update_on_fail_step_progress(self.step_index, idx, progress)
            }
        }
    }

    pub fn append_output(&self, text: String) {
        match self.on_fail_step_index {
            None => self.manager.append_step_output(self.step_index, text),
            Some(idx) => {
                self.manager
                    .append_on_fail_step_output(self.step_index, idx, text)
            }
        }
    }

    pub fn add_error(&self, error: String) {
        match self.on_fail_step_index {
            None => self.manager.add_step_error(self.step_index, error),
            Some(idx) => {
                self.manager
                    .add_on_fail_step_error(self.step_index, idx, error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn create_test_state() -> ExecutionState {
        ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![
                StepExecState {
                    index: 0,
                    task_description: "Step 1".to_string(),
                    status: StepStatus::Pending,
                    progress: None,
                    output: String::new(),
                    errors: Vec::new(),
                    on_fail_steps: vec![OnFailStepExecState {
                        index: 0,
                        task_description: "Recovery 1".to_string(),
                        status: StepStatus::Pending,
                        progress: None,
                        output: String::new(),
                        errors: Vec::new(),
                    }],
                },
                StepExecState {
                    index: 1,
                    task_description: "Step 2".to_string(),
                    status: StepStatus::Pending,
                    progress: None,
                    output: String::new(),
                    errors: Vec::new(),
                    on_fail_steps: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn test_snapshot_returns_current_state() {
        let (tx, _rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.status, ExecutionStatus::Idle);
        assert_eq!(snapshot.steps.len(), 2);
    }

    #[test]
    fn test_update_execution_status_emits_diff() {
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);

        manager.update_execution_status(ExecutionStatus::Running);

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.status, ExecutionStatus::Running);

        let diff = rx.try_recv().unwrap();
        assert!(matches!(
            diff,
            StateDiff::ExecutionStatusChanged {
                status: ExecutionStatus::Running
            }
        ));
    }

    #[test]
    fn test_update_step_status_emits_diff() {
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);

        manager.update_step_status(0, StepStatus::Running);

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[0].status, StepStatus::Running);
        assert_eq!(snapshot.steps[1].status, StepStatus::Pending);

        let diff = rx.try_recv().unwrap();
        assert!(matches!(
            diff,
            StateDiff::StepStatusChanged {
                step_index: 0,
                status: StepStatus::Running,
            }
        ));
    }

    #[test]
    fn test_append_step_output() {
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);

        manager.append_step_output(0, "line 1".to_string());
        manager.append_step_output(0, "line 2".to_string());

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[0].output, "line 1\nline 2");

        let diff1 = rx.try_recv().unwrap();
        assert!(matches!(diff1, StateDiff::StepOutputAppended { step_index: 0, .. }));
        let diff2 = rx.try_recv().unwrap();
        assert!(matches!(diff2, StateDiff::StepOutputAppended { step_index: 0, .. }));
    }

    #[test]
    fn test_add_step_error() {
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);

        manager.add_step_error(1, "something failed".to_string());

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[1].errors, vec!["something failed"]);

        let diff = rx.try_recv().unwrap();
        assert!(matches!(diff, StateDiff::StepErrorAdded { step_index: 1, .. }));
    }

    #[test]
    fn test_on_fail_step_status_update() {
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);

        manager.update_on_fail_step_status(0, 0, StepStatus::Running);

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[0].on_fail_steps[0].status, StepStatus::Running);

        let diff = rx.try_recv().unwrap();
        assert!(matches!(
            diff,
            StateDiff::OnFailStepStatusChanged {
                step_index: 0,
                on_fail_step_index: 0,
                status: StepStatus::Running,
            }
        ));
    }

    #[test]
    fn test_task_tracker_for_step() {
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);
        let tracker = TaskTracker::for_step(&manager, 0);

        tracker.append_output("hello".to_string());

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[0].output, "hello");

        let diff = rx.try_recv().unwrap();
        assert!(matches!(diff, StateDiff::StepOutputAppended { step_index: 0, .. }));
    }

    #[test]
    fn test_task_tracker_for_on_fail_step() {
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);
        let tracker = TaskTracker::for_on_fail_step(&manager, 0, 0);

        tracker.append_output("recovery output".to_string());

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[0].on_fail_steps[0].output, "recovery output");

        let diff = rx.try_recv().unwrap();
        assert!(matches!(
            diff,
            StateDiff::OnFailStepOutputAppended {
                step_index: 0,
                on_fail_step_index: 0,
                ..
            }
        ));
    }

    #[test]
    fn test_update_step_progress() {
        // Given
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);
        let progress = TaskProgress::RemoteSudo {
            command: "apt-get update".to_string(),
            output: "Reading package lists...".to_string(),
        };

        // When
        manager.update_step_progress(0, progress.clone());

        // Then
        let snapshot = manager.snapshot();
        assert!(matches!(
            snapshot.steps[0].progress.as_ref().unwrap(),
            TaskProgress::RemoteSudo { .. }
        ));
        let diff = rx.try_recv().unwrap();
        assert!(matches!(diff, StateDiff::StepProgressUpdated { step_index: 0, .. }));
    }

    #[test]
    fn test_update_on_fail_step_progress() {
        // Given
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);
        let progress = TaskProgress::SftpCopy {
            source: "/local/file".to_string(),
            destination: "/remote/file".to_string(),
            bytes_transferred: 512,
            bytes_total: 1024,
            elapsed_ms: 300,
        };

        // When
        manager.update_on_fail_step_progress(0, 0, progress.clone());

        // Then
        let snapshot = manager.snapshot();
        assert!(matches!(
            snapshot.steps[0].on_fail_steps[0].progress.as_ref().unwrap(),
            TaskProgress::SftpCopy { .. }
        ));
        let diff = rx.try_recv().unwrap();
        assert!(matches!(
            diff,
            StateDiff::OnFailStepProgressUpdated {
                step_index: 0,
                on_fail_step_index: 0,
                ..
            }
        ));
    }

    #[test]
    fn test_append_on_fail_step_output() {
        // Given
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);

        // When
        manager.append_on_fail_step_output(0, 0, "line 1".to_string());
        manager.append_on_fail_step_output(0, 0, "line 2".to_string());

        // Then
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[0].on_fail_steps[0].output, "line 1\nline 2");
        let diff1 = rx.try_recv().unwrap();
        assert!(matches!(diff1, StateDiff::OnFailStepOutputAppended { step_index: 0, on_fail_step_index: 0, .. }));
        let diff2 = rx.try_recv().unwrap();
        assert!(matches!(diff2, StateDiff::OnFailStepOutputAppended { step_index: 0, on_fail_step_index: 0, .. }));
    }

    #[test]
    fn test_add_on_fail_step_error() {
        // Given
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);

        // When
        manager.add_on_fail_step_error(0, 0, "recovery failed".to_string());

        // Then
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[0].on_fail_steps[0].errors, vec!["recovery failed"]);
        let diff = rx.try_recv().unwrap();
        assert!(matches!(
            diff,
            StateDiff::OnFailStepErrorAdded {
                step_index: 0,
                on_fail_step_index: 0,
                ..
            }
        ));
    }

    #[test]
    fn test_task_tracker_update_progress_for_step() {
        // Given
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);
        let tracker = TaskTracker::for_step(&manager, 0);
        let progress = TaskProgress::RemoteSudo {
            command: "ls".to_string(),
            output: "file.txt".to_string(),
        };

        // When
        tracker.update_progress(progress);

        // Then
        let snapshot = manager.snapshot();
        assert!(snapshot.steps[0].progress.is_some());
        let diff = rx.try_recv().unwrap();
        assert!(matches!(diff, StateDiff::StepProgressUpdated { step_index: 0, .. }));
    }

    #[test]
    fn test_task_tracker_update_progress_for_on_fail_step() {
        // Given
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);
        let tracker = TaskTracker::for_on_fail_step(&manager, 0, 0);
        let progress = TaskProgress::RemoteSudo {
            command: "recovery".to_string(),
            output: "done".to_string(),
        };

        // When
        tracker.update_progress(progress);

        // Then
        let snapshot = manager.snapshot();
        assert!(snapshot.steps[0].on_fail_steps[0].progress.is_some());
        let diff = rx.try_recv().unwrap();
        assert!(matches!(diff, StateDiff::OnFailStepProgressUpdated { step_index: 0, on_fail_step_index: 0, .. }));
    }

    #[test]
    fn test_task_tracker_add_error_for_step() {
        // Given
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);
        let tracker = TaskTracker::for_step(&manager, 0);

        // When
        tracker.add_error("step failed".to_string());

        // Then
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[0].errors, vec!["step failed"]);
        let diff = rx.try_recv().unwrap();
        assert!(matches!(diff, StateDiff::StepErrorAdded { step_index: 0, .. }));
    }

    #[test]
    fn test_task_tracker_add_error_for_on_fail_step() {
        // Given
        let (tx, rx) = mpsc::channel();
        let manager = ExecutionStateManager::new(create_test_state(), tx);
        let tracker = TaskTracker::for_on_fail_step(&manager, 0, 0);

        // When
        tracker.add_error("on-fail failed".to_string());

        // Then
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.steps[0].on_fail_steps[0].errors, vec!["on-fail failed"]);
        let diff = rx.try_recv().unwrap();
        assert!(matches!(diff, StateDiff::OnFailStepErrorAdded { step_index: 0, on_fail_step_index: 0, .. }));
    }
}
