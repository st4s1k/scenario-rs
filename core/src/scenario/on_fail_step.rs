use crate::{
    scenario::{errors::OnFailError, task::Task, variables::Variables},
    session::Session,
    state::{ExecutionStateManager, TaskTracker},
    state::types::StepStatus,
    trace::ScenarioEvent,
};
use tracing::{debug, instrument};

#[derive(Clone, Debug)]
pub struct OnFailStep {
    pub(crate) index: usize,
    pub(crate) task: Task,
}

impl From<(usize, Task)> for OnFailStep {
    fn from((index, task): (usize, Task)) -> Self {
        Self { index, task }
    }
}

impl OnFailStep {
    /// Returns the index of the on-fail step
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the task associated with this on-fail step
    pub fn task(&self) -> &Task {
        &self.task
    }

    /// Execute the on-fail step
    #[instrument(
        name = "on_fail_step",
        skip_all,
        fields(on_fail_step.index = self.index)
    )]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        state_manager: Option<&ExecutionStateManager>,
        parent_step_index: usize,
    ) -> Result<(), OnFailError> {
        debug!(
            scenario.event = ScenarioEvent::OnFailStepStarted.as_str(),
            task.description = self.task.description()
        );

        if let Some(sm) = state_manager {
            sm.update_on_fail_step_status(parent_step_index, self.index, StepStatus::Running);
        }

        let tracker =
            state_manager.map(|sm| TaskTracker::for_on_fail_step(sm, parent_step_index, self.index));

        let result = match &self.task {
            Task::RemoteSudo { remote_sudo, .. } => remote_sudo
                .execute(session, variables, tracker.as_ref())
                .map_err(OnFailError::CannotOnFailRemoteSudo)
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                }),
            Task::SftpCopy { sftp_copy, .. } => sftp_copy
                .execute(session, variables, tracker.as_ref())
                .map_err(OnFailError::CannotOnFailSftpCopy)
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                }),
        };

        if result.is_ok() {
            debug!(scenario.event = ScenarioEvent::OnFailStepCompleted.as_str());
            if let Some(sm) = state_manager {
                sm.update_on_fail_step_status(
                    parent_step_index,
                    self.index,
                    StepStatus::Completed,
                );
            }
        } else if let Some(sm) = state_manager {
            sm.update_on_fail_step_status(parent_step_index, self.index, StepStatus::Failed);
            if let Err(ref e) = result {
                sm.add_on_fail_step_error(parent_step_index, self.index, e.to_string());
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        scenario::{
            remote_sudo::RemoteSudo,
            sftp_copy::SftpCopy,
        },
        session::{Channel, Session, SessionType, Sftp},
        state::types::{
            ExecutionState, ExecutionStatus, OnFailStepExecState, StepExecState, StepStatus,
        },
        utils::{ArcMutex, Wrap},
    };
    use std::sync::mpsc;

    struct SuccessChannel;
    impl Channel for SuccessChannel {
        fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> { Ok(()) }
        fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
            output.push_str("ok");
            Ok(2)
        }
        fn exit_status(&self) -> Result<i32, ssh2::Error> { Ok(0) }
    }

    struct FailChannel;
    impl Channel for FailChannel {
        fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
            Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
        }
        fn read_to_string(&mut self, _: &mut String) -> Result<usize, ssh2::Error> { Ok(0) }
        fn exit_status(&self) -> Result<i32, ssh2::Error> { Ok(0) }
    }

    struct TestWrite;
    impl crate::session::Write for TestWrite {
        fn write_all(&mut self, _buf: &[u8]) -> Result<(), ssh2::Error> { Ok(()) }
    }

    struct TestSftp;
    impl Sftp for TestSftp {
        fn create(&self, _path: &std::path::Path) -> Result<Box<dyn crate::session::Write>, ssh2::Error> {
            Ok(Box::new(TestWrite))
        }
    }

    struct FailSftp;
    impl Sftp for FailSftp {
        fn create(&self, _path: &std::path::Path) -> Result<Box<dyn crate::session::Write>, ssh2::Error> {
            Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
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

    fn fail_channel_session() -> Session {
        Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(FailChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        }
    }

    fn fail_sftp_session() -> Session {
        Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(SuccessChannel),
                sftp: ArcMutex::wrap(FailSftp),
            },
        }
    }

    fn make_remote_sudo_step(index: usize) -> OnFailStep {
        OnFailStep {
            index,
            task: Task::RemoteSudo {
                description: "test sudo".into(),
                error_message: "sudo failed".into(),
                remote_sudo: RemoteSudo { command: "echo test".into() },
            },
        }
    }

    fn make_sftp_step(index: usize) -> OnFailStep {
        OnFailStep {
            index,
            task: Task::SftpCopy {
                description: "test copy".into(),
                error_message: "copy failed".into(),
                sftp_copy: SftpCopy {
                    source_path: "source.txt".into(),
                    destination_path: "dest.txt".into(),
                },
            },
        }
    }

    fn create_state_manager() -> (ExecutionStateManager, mpsc::Receiver<crate::state::types::StateDiff>) {
        let (tx, rx) = mpsc::channel();
        let state = ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![StepExecState {
                index: 0,
                task_description: "parent step".into(),
                status: StepStatus::Pending,
                progress: None,
                output: String::new(),
                errors: Vec::new(),
                on_fail_steps: vec![
                    OnFailStepExecState {
                        index: 0,
                        task_description: "recovery".into(),
                        status: StepStatus::Pending,
                        progress: None,
                        output: String::new(),
                        errors: Vec::new(),
                    },
                ],
            }],
        };
        (ExecutionStateManager::new(state, tx), rx)
    }

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::TRACE)
            .try_init();
    }

    #[test]
    fn test_execute_remote_sudo_success() {
        init_tracing();

        // Given
        let step = make_remote_sudo_step(0);
        let session = success_session();
        let variables = Variables::default();

        // When
        let result = step.execute(&session, &variables, None, 0);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_sftp_copy_success() {
        // Given
        let step = make_sftp_step(0);
        let session = success_session();
        let variables = Variables::default();

        // When
        let result = step.execute(&session, &variables, None, 0);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_remote_sudo_failure() {
        // Given
        let step = make_remote_sudo_step(0);
        let session = fail_channel_session();
        let variables = Variables::default();

        // When
        let result = step.execute(&session, &variables, None, 0);

        // Then
        assert!(matches!(result, Err(OnFailError::CannotOnFailRemoteSudo(_))));
    }

    #[test]
    fn test_execute_sftp_copy_failure() {
        // Given
        let step = make_sftp_step(0);
        let session = fail_sftp_session();
        let variables = Variables::default();

        // When
        let result = step.execute(&session, &variables, None, 0);

        // Then
        assert!(matches!(result, Err(OnFailError::CannotOnFailSftpCopy(_))));
    }

    #[test]
    fn test_state_manager_receives_running_then_completed() {
        // Given
        let step = make_remote_sudo_step(0);
        let session = success_session();
        let variables = Variables::default();
        let (sm, rx) = create_state_manager();

        // When
        let result = step.execute(&session, &variables, Some(&sm), 0);

        // Then
        assert!(result.is_ok());
        let diffs: Vec<_> = rx.try_iter().collect();
        assert!(diffs.len() >= 2, "expected at least Running + Completed diffs, got {}", diffs.len());
        assert!(matches!(&diffs[0],
            crate::state::types::StateDiff::OnFailStepStatusChanged { step_index: 0, on_fail_step_index: 0, status: StepStatus::Running }
        ));
        assert!(matches!(diffs.last().unwrap(),
            crate::state::types::StateDiff::OnFailStepStatusChanged { step_index: 0, on_fail_step_index: 0, status: StepStatus::Completed }
        ));
    }

    #[test]
    fn test_state_manager_receives_running_then_failed_on_error() {
        // Given
        let step = make_remote_sudo_step(0);
        let session = fail_channel_session();
        let variables = Variables::default();
        let (sm, rx) = create_state_manager();

        // When
        let result = step.execute(&session, &variables, Some(&sm), 0);

        // Then
        assert!(result.is_err());
        let diffs: Vec<_> = rx.try_iter().collect();
        assert!(diffs.len() >= 2, "expected at least Running + Failed diffs, got {}", diffs.len());

        assert!(matches!(&diffs[0],
            crate::state::types::StateDiff::OnFailStepStatusChanged { step_index: 0, on_fail_step_index: 0, status: StepStatus::Running }
        ));
        let has_failed = diffs.iter().any(|d| matches!(d,
            crate::state::types::StateDiff::OnFailStepStatusChanged { status: StepStatus::Failed, .. }
        ));
        let has_error = diffs.iter().any(|d| matches!(d,
            crate::state::types::StateDiff::OnFailStepErrorAdded { .. }
        ));
        assert!(has_failed, "expected Failed status diff");
        assert!(has_error, "expected error diff");
    }

    #[test]
    fn test_from_tuple() {
        // Given & When
        let task = Task::RemoteSudo {
            description: "desc".into(),
            error_message: "err".into(),
            remote_sudo: RemoteSudo { command: "cmd".into() },
        };
        let step = OnFailStep::from((2, task));

        // Then
        assert_eq!(step.index(), 2);
        assert_eq!(step.task().description(), "desc");
    }
}
