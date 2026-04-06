use scenario_rs_core::{
    scenario::{
        on_fail_step::OnFailStep,
        on_fail_steps::OnFailSteps,
        remote_sudo::RemoteSudo,
        sftp_copy::SftpCopy,
        step::Step,
        steps::Steps,
        task::Task,
        variables::Variables,
    },
    session::{Channel, Session, SessionType, Sftp},
    state::{
        types::{
            ExecutionState, ExecutionStatus, OnFailStepExecState, StateDiff, StepExecState,
            StepStatus, TaskProgress,
        },
        ExecutionStateManager,
    },
    utils::{ArcMutex, Wrap},
};
use std::sync::mpsc;

struct SuccessChannel;
impl Channel for SuccessChannel {
    fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
        Ok(())
    }
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
        output.push_str("command output");
        Ok(14)
    }
    fn exit_status(&self) -> Result<i32, ssh2::Error> {
        Ok(0)
    }
}

struct FailChannel;
impl Channel for FailChannel {
    fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
        Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
    }
    fn read_to_string(&mut self, _: &mut String) -> Result<usize, ssh2::Error> {
        Ok(0)
    }
    fn exit_status(&self) -> Result<i32, ssh2::Error> {
        Ok(0)
    }
}

struct TestWrite;
impl scenario_rs_core::session::Write for TestWrite {
    fn write_all(&mut self, _buf: &[u8]) -> Result<(), ssh2::Error> {
        Ok(())
    }
}

struct TestSftp;
impl Sftp for TestSftp {
    fn create(
        &self,
        _path: &std::path::Path,
    ) -> Result<Box<dyn scenario_rs_core::session::Write>, ssh2::Error> {
        Ok(Box::new(TestWrite))
    }
}

fn success_session() -> Session {
    Session {
        inner: SessionType::Test {
            channel: ArcMutex::wrap(SuccessChannel),
            sftp: ArcMutex::wrap(TestSftp),
        },
    }
}

fn fail_session() -> Session {
    Session {
        inner: SessionType::Test {
            channel: ArcMutex::wrap(FailChannel),
            sftp: ArcMutex::wrap(TestSftp),
        },
    }
}

fn make_sudo_task(desc: &str, cmd: &str) -> Task {
    Task::RemoteSudo {
        description: desc.into(),
        error_message: format!("{desc} failed"),
        remote_sudo: RemoteSudo {
            command: cmd.into(),
        },
    }
}

fn make_sftp_task(desc: &str) -> Task {
    Task::SftpCopy {
        description: desc.into(),
        error_message: format!("{desc} failed"),
        sftp_copy: SftpCopy {
            source_path: "local.txt".into(),
            destination_path: "remote.txt".into(),
        },
    }
}

fn step_exec_state(index: usize, desc: &str, on_fail_count: usize) -> StepExecState {
    StepExecState {
        index,
        task_description: desc.into(),
        status: StepStatus::Pending,
        progress: None,
        output: String::new(),
        errors: Vec::new(),
        on_fail_steps: (0..on_fail_count)
            .map(|i| OnFailStepExecState {
                index: i,
                task_description: format!("recovery-{i}"),
                status: StepStatus::Pending,
                progress: None,
                output: String::new(),
                errors: Vec::new(),
            })
            .collect(),
    }
}

fn collect_diffs(rx: &mpsc::Receiver<StateDiff>) -> Vec<StateDiff> {
    rx.try_iter().collect()
}

#[test]
fn single_step_success_emits_running_then_completed() {
    // Given
    let steps = Steps::from(vec![Step {
        index: 0,
        task: make_sudo_task("echo", "echo hi"),
        on_fail_steps: OnFailSteps::default(),
    }]);

    let (tx, rx) = mpsc::channel();
    let sm = ExecutionStateManager::new(
        ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![step_exec_state(0, "echo", 0)],
        },
        tx,
    );

    // When
    let result = steps.execute(&success_session(), &Variables::default(), Some(&sm));

    // Then
    assert!(result.is_ok());
    let diffs = collect_diffs(&rx);
    let status_diffs: Vec<_> = diffs
        .iter()
        .filter_map(|d| match d {
            StateDiff::StepStatusChanged { step_index, status } => Some((*step_index, status.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(status_diffs.len(), 2);
    assert_eq!(status_diffs[0], (0, StepStatus::Running));
    assert_eq!(status_diffs[1], (0, StepStatus::Completed));
    let snap = sm.snapshot();
    assert_eq!(snap.steps[0].status, StepStatus::Completed);
}

#[test]
fn multi_step_success_all_complete() {
    // Given
    let steps = Steps::from(vec![
        Step { index: 0, task: make_sudo_task("step1", "echo 1"), on_fail_steps: OnFailSteps::default() },
        Step { index: 1, task: make_sudo_task("step2", "echo 2"), on_fail_steps: OnFailSteps::default() },
        Step { index: 2, task: make_sudo_task("step3", "echo 3"), on_fail_steps: OnFailSteps::default() },
    ]);

    let (tx, rx) = mpsc::channel();
    let sm = ExecutionStateManager::new(
        ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![
                step_exec_state(0, "step1", 0),
                step_exec_state(1, "step2", 0),
                step_exec_state(2, "step3", 0),
            ],
        },
        tx,
    );

    // When
    let result = steps.execute(&success_session(), &Variables::default(), Some(&sm));

    // Then
    assert!(result.is_ok());
    let snap = sm.snapshot();
    for (i, step) in snap.steps.iter().enumerate() {
        assert_eq!(step.status, StepStatus::Completed, "step {i} should be Completed");
    }
    let diffs = collect_diffs(&rx);
    let status_diffs: Vec<_> = diffs
        .iter()
        .filter_map(|d| match d {
            StateDiff::StepStatusChanged { step_index, status } => Some((*step_index, status.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(status_diffs.len(), 6); // 3 steps × 2 transitions
    assert_eq!(status_diffs[0], (0, StepStatus::Running));
    assert_eq!(status_diffs[1], (0, StepStatus::Completed));
    assert_eq!(status_diffs[2], (1, StepStatus::Running));
    assert_eq!(status_diffs[3], (1, StepStatus::Completed));
    assert_eq!(status_diffs[4], (2, StepStatus::Running));
    assert_eq!(status_diffs[5], (2, StepStatus::Completed));
}

#[test]
fn step_failure_stops_execution_and_remaining_stay_pending() {
    // Given
    let steps = Steps::from(vec![
        Step { index: 0, task: make_sudo_task("pass", "echo ok"), on_fail_steps: OnFailSteps::default() },
        Step { index: 1, task: make_sudo_task("fail", "bad cmd"), on_fail_steps: OnFailSteps::default() },
        Step { index: 2, task: make_sudo_task("skip", "echo skip"), on_fail_steps: OnFailSteps::default() },
    ]);

    let (tx, rx) = mpsc::channel();
    let sm = ExecutionStateManager::new(
        ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![
                step_exec_state(0, "pass", 0),
                step_exec_state(1, "fail", 0),
                step_exec_state(2, "skip", 0),
            ],
        },
        tx,
    );

    // When
    let result = steps.execute(&fail_session(), &Variables::default(), Some(&sm));

    // Then
    assert!(result.is_err());
    let snap = sm.snapshot();
    assert_eq!(snap.steps[0].status, StepStatus::Failed);
    assert_eq!(snap.steps[1].status, StepStatus::Pending);
    assert_eq!(snap.steps[2].status, StepStatus::Pending);
    assert!(!snap.steps[0].errors.is_empty());
    let diffs = collect_diffs(&rx);
    let has_failed = diffs.iter().any(|d| matches!(d,
        StateDiff::StepStatusChanged { step_index: 0, status: StepStatus::Failed }
    ));
    assert!(has_failed, "expected StepStatusChanged to Failed for step 0");
}

#[test]
fn on_fail_steps_execute_after_step_failure() {
    // Given
    let steps = Steps::from(vec![Step {
        index: 0,
        task: make_sudo_task("deploy", "deploy.sh"),
        on_fail_steps: OnFailSteps::from(vec![
            OnFailStep::from((0, make_sudo_task("rollback", "rollback.sh"))),
            OnFailStep::from((1, make_sudo_task("notify", "notify.sh"))),
        ]),
    }]);

    let (tx, rx) = mpsc::channel();
    let sm = ExecutionStateManager::new(
        ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![step_exec_state(0, "deploy", 2)],
        },
        tx,
    );

    // When
    let result = steps.execute(&fail_session(), &Variables::default(), Some(&sm));

    // Then
    assert!(result.is_err());
    let snap = sm.snapshot();
    assert_eq!(snap.steps[0].status, StepStatus::Failed);

    let diffs = collect_diffs(&rx);
    let on_fail_running = diffs.iter().any(|d| matches!(d,
        StateDiff::OnFailStepStatusChanged { step_index: 0, on_fail_step_index: 0, status: StepStatus::Running }
    ));
    assert!(on_fail_running, "on-fail step 0 should have been attempted");
}

#[test]
fn on_fail_steps_succeed_with_success_session_and_main_step_sftp_fail() {
    // Given
    struct FailSftp;
    impl Sftp for FailSftp {
        fn create(
            &self,
            _path: &std::path::Path,
        ) -> Result<Box<dyn scenario_rs_core::session::Write>, ssh2::Error> {
            Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
        }
    }

    let session = Session {
        inner: SessionType::Test {
            channel: ArcMutex::wrap(SuccessChannel),
            sftp: ArcMutex::wrap(FailSftp),
        },
    };

    let steps = Steps::from(vec![Step {
        index: 0,
        task: make_sftp_task("copy file"),
        on_fail_steps: OnFailSteps::from(vec![
            OnFailStep::from((0, make_sudo_task("cleanup", "rm /tmp/partial"))),
        ]),
    }]);

    let (tx, rx) = mpsc::channel();
    let sm = ExecutionStateManager::new(
        ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![step_exec_state(0, "copy file", 1)],
        },
        tx,
    );

    // When
    let result = steps.execute(&session, &Variables::default(), Some(&sm));

    // Then
    assert!(result.is_err());
    let snap = sm.snapshot();
    assert_eq!(snap.steps[0].status, StepStatus::Failed);
    let diffs = collect_diffs(&rx);
    let on_fail_completed = diffs.iter().any(|d| matches!(d,
        StateDiff::OnFailStepStatusChanged { step_index: 0, on_fail_step_index: 0, status: StepStatus::Completed }
    ));
    assert!(on_fail_completed, "on-fail step should have Completed");
}

#[test]
fn remote_sudo_step_emits_progress_and_output() {
    // Given & When
    let steps = Steps::from(vec![Step {
        index: 0,
        task: make_sudo_task("cmd", "echo hello"),
        on_fail_steps: OnFailSteps::default(),
    }]);

    let (tx, rx) = mpsc::channel();
    let sm = ExecutionStateManager::new(
        ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![step_exec_state(0, "cmd", 0)],
        },
        tx,
    );

    steps.execute(&success_session(), &Variables::default(), Some(&sm)).unwrap();

    // Then
    let diffs = collect_diffs(&rx);
    let has_progress = diffs.iter().any(|d| matches!(d,
        StateDiff::StepProgressUpdated { step_index: 0, progress: TaskProgress::RemoteSudo { .. } }
    ));
    assert!(has_progress, "expected RemoteSudo progress update");
    let has_output = diffs.iter().any(|d| matches!(d,
        StateDiff::StepOutputAppended { step_index: 0, .. }
    ));
    assert!(has_output, "expected output append");
}

#[test]
fn empty_steps_succeed_immediately() {
    // Given & When & Then
    let steps = Steps::default();

    let (tx, rx) = mpsc::channel();
    let sm = ExecutionStateManager::new(
        ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![],
        },
        tx,
    );

    let result = steps.execute(&success_session(), &Variables::default(), Some(&sm));
    assert!(result.is_ok());

    let diffs = collect_diffs(&rx);
    assert!(diffs.is_empty(), "empty steps should emit no diffs");
}

#[test]
fn execution_without_state_manager_still_works() {
    // Given & When & Then
    let steps = Steps::from(vec![Step {
        index: 0,
        task: make_sudo_task("cmd", "echo"),
        on_fail_steps: OnFailSteps::default(),
    }]);
    let result = steps.execute(&success_session(), &Variables::default(), None);
    assert!(result.is_ok());
}
