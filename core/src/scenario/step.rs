//! Step execution handling for scenarios.
//!
//! This module provides functionality for executing individual steps
//! in a scenario, including regular tasks and fallback steps for error handling.

use crate::{
    scenario::{
        errors::StepError, on_fail_steps::OnFailSteps, task::Task, variables::Variables,
    },
    session::Session,
    state::{ExecutionStateManager, TaskTracker},
    state::types::StepStatus,
    trace::ScenarioEvent,
};
use tracing::{debug, instrument};

/// A single step in a scenario: a task with optional on-fail recovery steps.
#[derive(Clone, Debug)]
pub struct Step {
    pub index: usize,
    pub task: Task,
    pub on_fail_steps: OnFailSteps,
}

impl Step {
    /// Creates a new step with the given index, task, and on-fail recovery steps.
    pub fn new(index: usize, task: Task, on_fail_steps: OnFailSteps) -> Self {
        Step {
            index,
            task,
            on_fail_steps,
        }
    }
}

impl Step {
    /// Returns the index of the step in the scenario.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns a reference to the step's task.
    pub fn task(&self) -> &Task {
        &self.task
    }

    /// Returns a reference to the step's on-fail steps.
    pub fn on_fail_steps(&self) -> &OnFailSteps {
        &self.on_fail_steps
    }

    #[instrument(
        name = "step"
        skip_all,
        fields(step.index = self.index)
    )]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        state_manager: Option<&ExecutionStateManager>,
    ) -> Result<(), StepError> {
        let description = self.task.description().to_string();

        debug!(
            scenario.event = ScenarioEvent::StepStarted.as_str(),
            task.description = description
        );

        if let Some(sm) = state_manager {
            sm.update_step_status(self.index, StepStatus::Running);
        }

        let error_message = self.task.error_message().to_string();
        let tracker = state_manager.map(|sm| TaskTracker::for_step(sm, self.index));

        let task_result = match &self.task {
            Task::RemoteSudo { remote_sudo, .. } => remote_sudo
                .execute(session, variables, tracker.as_ref())
                .map_err(|error| {
                    StepError::CannotExecuteRemoteSudoCommand(error, error_message.clone())
                })
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                }),
            Task::SftpCopy { sftp_copy, .. } => sftp_copy
                .execute(session, variables, tracker.as_ref())
                .map_err(|error| {
                    StepError::CannotExecuteSftpCopyCommand(error, error_message.clone())
                })
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                }),
        };

        if let Err(error) = task_result {
            if let Some(sm) = state_manager {
                sm.update_step_status(self.index, StepStatus::Failed);
                sm.add_step_error(self.index, error.to_string());
            }
            self.execute_on_fail_steps(session, &variables, state_manager)?;
            return Err(error);
        }

        debug!(scenario.event = ScenarioEvent::StepCompleted.as_str());
        if let Some(sm) = state_manager {
            sm.update_step_status(self.index, StepStatus::Completed);
        }
        Ok(())
    }

    fn execute_on_fail_steps(
        &self,
        session: &Session,
        variables: &Variables,
        state_manager: Option<&ExecutionStateManager>,
    ) -> Result<(), StepError> {
        self.on_fail_steps
            .execute(session, variables, state_manager, self.index)
            .map_err(StepError::CannotExecuteOnFailSteps)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                error
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::task::{RemoteSudoTaskConfig, SftpCopyTaskConfig},
        scenario::{errors::StepError, on_fail_steps::OnFailSteps, on_fail_step::OnFailStep, step::Step, task::Task, variables::Variables},
        session::{Channel, Session, SessionType, Sftp, SshError},
        utils::{ArcMutex, Wrap},
    };

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::TRACE)
            .try_init();
    }

    #[test]
    fn test_step_new() {
        // Given
        let task = create_remote_sudo_task();

        // When
        let step = Step::new(0, task, OnFailSteps::default());

        // Then
        assert_eq!(step.index(), 0);
        assert_eq!(step.task().description(), "Test task 1");
        assert!(step.on_fail_steps().is_empty());
    }

    #[test]
    fn test_step_new_with_on_fail() {
        // Given
        let task = create_remote_sudo_task();
        let on_fail = OnFailSteps::from(vec![OnFailStep::from((0, create_sftp_copy_task()))]);

        // When
        let step = Step::new(3, task, on_fail);

        // Then
        assert_eq!(step.index(), 3);
        assert_eq!(step.on_fail_steps().len(), 1);
    }

    #[test]
    fn test_step_accessors() {
        // Given
        let on_fail = OnFailSteps::from(vec![OnFailStep::from((0, create_sftp_copy_task()))]);
        let step = Step::new(5, create_remote_sudo_task(), on_fail);

        // Then
        assert_eq!(step.index(), 5);
        assert_eq!(step.task().description(), "Test task 1");
        assert_eq!(step.on_fail_steps().len(), 1);
    }

    #[test]
    fn test_step_clone() {
        // Given
        let on_fail = OnFailSteps::from(vec![OnFailStep::from((0, create_sftp_copy_task()))]);
        let original = Step::new(0, create_remote_sudo_task(), on_fail);

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(cloned.task().description(), original.task().description());
        assert_eq!(cloned.on_fail_steps().len(), original.on_fail_steps().len());
    }

    #[test]
    fn test_step_execute_success() {
        init_tracing();

        struct TestChannel;
        impl Channel for TestChannel {
            fn exec(&mut self, _: &str) -> Result<(), SshError> { Ok(()) }
            fn read_to_string(&mut self, _: &mut String) -> Result<usize, SshError> { Ok(0) }
            fn exit_status(&self) -> Result<i32, SshError> { Ok(0) }
        }
        struct TestWrite;
        impl crate::session::Write for TestWrite {
            fn write_all(&mut self, _: &[u8]) -> Result<(), SshError> { Ok(()) }
        }
        struct TestSftp;
        impl Sftp for TestSftp {
            fn create(&self, _: &std::path::Path) -> Result<Box<dyn crate::session::Write>, SshError> {
                Ok(Box::new(TestWrite))
            }
        }

        // Given
        let step = Step::new(0, create_remote_sudo_task(), OnFailSteps::default());
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(TestChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = step.execute(&session, &variables, None);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_step_execute_success_with_state_manager() {
        init_tracing();

        use crate::state::{
            types::{ExecutionState, ExecutionStatus, StepExecState, StepStatus},
            ExecutionStateManager,
        };
        use std::sync::mpsc;

        struct TestChannel;
        impl Channel for TestChannel {
            fn exec(&mut self, _: &str) -> Result<(), SshError> { Ok(()) }
            fn read_to_string(&mut self, _: &mut String) -> Result<usize, SshError> { Ok(0) }
            fn exit_status(&self) -> Result<i32, SshError> { Ok(0) }
        }
        struct TestWrite;
        impl crate::session::Write for TestWrite {
            fn write_all(&mut self, _: &[u8]) -> Result<(), SshError> { Ok(()) }
        }
        struct TestSftp;
        impl Sftp for TestSftp {
            fn create(&self, _: &std::path::Path) -> Result<Box<dyn crate::session::Write>, SshError> {
                Ok(Box::new(TestWrite))
            }
        }

        // Given
        let step = Step::new(0, create_remote_sudo_task(), OnFailSteps::default());
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(TestChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        let (tx, _rx) = mpsc::channel();
        let state = ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![StepExecState {
                index: 0,
                task_description: "test".into(),
                status: StepStatus::Pending,
                progress: None,
                output: String::new(),
                errors: Vec::new(),
                on_fail_steps: Vec::new(),
            }],
        };
        let sm = ExecutionStateManager::new(state, tx);

        // When
        let result = step.execute(&session, &variables, Some(&sm));

        // Then
        assert!(result.is_ok());
        let snapshot = sm.snapshot();
        assert_eq!(snapshot.steps[0].status, StepStatus::Completed);
    }

    #[test]
    fn test_step_execute_remote_sudo_failure() {
        init_tracing();

        struct FailChannel;
        impl Channel for FailChannel {
            fn exec(&mut self, _: &str) -> Result<(), SshError> {
                Err(SshError::new("exec failed"))
            }
            fn read_to_string(&mut self, _: &mut String) -> Result<usize, SshError> { Ok(0) }
            fn exit_status(&self) -> Result<i32, SshError> { Ok(0) }
        }
        struct TestWrite;
        impl crate::session::Write for TestWrite {
            fn write_all(&mut self, _: &[u8]) -> Result<(), SshError> { Ok(()) }
        }
        struct TestSftp;
        impl Sftp for TestSftp {
            fn create(&self, _: &std::path::Path) -> Result<Box<dyn crate::session::Write>, SshError> {
                Ok(Box::new(TestWrite))
            }
        }

        // Given
        let step = Step::new(0, create_remote_sudo_task(), OnFailSteps::default());
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(FailChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = step.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(StepError::CannotExecuteRemoteSudoCommand(_, _))
        ));
    }

    #[test]
    fn test_step_execute_failure_triggers_on_fail_steps() {
        init_tracing();

        // Given
        let on_fail_task = create_sftp_copy_task();
        let step = Step {
            index: 0,
            task: create_remote_sudo_task(),
            on_fail_steps: OnFailSteps::from(vec![OnFailStep::from((0, on_fail_task))]),
        };
        let session = Session {
            inner: SessionType::FailSession("channel failed".to_string()),
        };
        let variables = Variables::default();

        // When
        let result = step.execute(&session, &variables, None);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_step_execute_on_fail_steps_also_fail() {
        init_tracing();

        use crate::scenario::sftp_copy::SftpCopy;

        struct FailChannel;
        impl Channel for FailChannel {
            fn exec(&mut self, _: &str) -> Result<(), SshError> {
                Err(SshError::new("exec failed"))
            }
            fn read_to_string(&mut self, _: &mut String) -> Result<usize, SshError> { Ok(0) }
            fn exit_status(&self) -> Result<i32, SshError> { Ok(0) }
        }
        struct TestWrite;
        impl crate::session::Write for TestWrite {
            fn write_all(&mut self, _: &[u8]) -> Result<(), SshError> { Ok(()) }
        }
        struct TestSftp;
        impl Sftp for TestSftp {
            fn create(&self, _: &std::path::Path) -> Result<Box<dyn crate::session::Write>, SshError> {
                Ok(Box::new(TestWrite))
            }
        }

        // Given
        let failing_on_fail_task = Task::SftpCopy {
            sftp_copy: SftpCopy {
                source_path: "{non-existent-var}".to_string(),
                destination_path: "/test/dest".to_string(),
            },
            description: "Failing on-fail task".to_string(),
            error_message: "On-fail failed".to_string(),
        };
        let step = Step {
            index: 0,
            task: create_remote_sudo_task(),
            on_fail_steps: OnFailSteps::from(vec![OnFailStep::from((0, failing_on_fail_task))]),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(FailChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = step.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(StepError::CannotExecuteOnFailSteps(_))
        ));
    }

    fn create_remote_sudo_task() -> Task {
        Task::from_remote_sudo(
            "task1",
            &RemoteSudoTaskConfig {
                command: "echo test".to_string(),
                description: Some("Test task 1".to_string()),
                error_message: Some("Task 1 failed".to_string()),
            },
        )
    }

    fn create_sftp_copy_task() -> Task {
        Task::from_sftp_copy(
            "task2",
            &SftpCopyTaskConfig {
                source: "/test/source".to_string(),
                destination: "/test/dest".to_string(),
                description: Some("Test task 2".to_string()),
                error_message: Some("Task 2 failed".to_string()),
            },
        )
    }
}
