use crate::{
    scenario::{errors::RemoteSudoError, variables::Variables},
    session::Session,
    state::TaskTracker,
    state::types::TaskProgress,
    trace::ScenarioEvent,
};
use std::fmt::Display;
use tracing::{debug, instrument};

fn log_scenario_error<E: Display>(error: E) -> E {
    let event = ScenarioEvent::Error;
    let err_display = error.to_string();
    debug!(scenario.event = event.as_str(), scenario.error = err_display);
    error
}

/// A remote command to be executed with sudo privileges.
#[derive(Debug, Clone)]
pub struct RemoteSudo {
    pub command: String,
}

impl RemoteSudo {
    pub fn new(command: String) -> Self {
        RemoteSudo { command }
    }

    /// Returns a reference to the command string
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Executes the sudo command on the remote session.
    #[instrument(
        name = "remote_sudo",
        skip_all,
        fields(remote_sudo.command)
    )]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        tracker: Option<&TaskTracker<'_>>,
    ) -> Result<(), RemoteSudoError> {
        let command = variables
            .resolve_placeholders(&self.command)
            .map_err(RemoteSudoError::CannotResolveCommandPlaceholders)
            .map_err(|error| {
                debug!(
                    scenario.event = ScenarioEvent::Error.as_str(),
                    scenario.error = %error,
                    remote_sudo.command = self.command
                );
                error
            })?;

        tracing::Span::current().record("remote_sudo.command", &command);

        debug!(scenario.event = ScenarioEvent::RemoteSudoStarted.as_str());

        let channel = session
            .channel_session()
            .map_err(RemoteSudoError::CannotEstablishSessionChannel)
            .map_err(log_scenario_error)?;

        channel
            .lock()
            .map_err(|_| RemoteSudoError::CannotGetALockOnChannel)
            .map_err(log_scenario_error)?
            .exec(&command)
            .map_err(RemoteSudoError::CannotExecuteRemoteCommand)
            .map_err(log_scenario_error)?;

        let mut output = String::new();
        channel
            .lock()
            .map_err(|_| RemoteSudoError::CannotGetALockOnChannel)
            .map_err(log_scenario_error)?
            .read_to_string(&mut output)
            .map_err(RemoteSudoError::CannotReadChannelOutput)
            .map_err(log_scenario_error)?;

        debug!(
            scenario.event = ScenarioEvent::RemoteSudoOutput.as_str(),
            remote_sudo.output = output
        );

        if let Some(tracker) = tracker {
            tracker.append_output(output.clone());
            tracker.update_progress(TaskProgress::RemoteSudo {
                command: command.clone(),
                output: output.clone(),
            });
        }

        let exit_status = channel
            .lock()
            .map_err(|_| RemoteSudoError::CannotGetALockOnChannel)
            .map_err(log_scenario_error)?
            .exit_status()
            .map_err(RemoteSudoError::CannotObtainRemoteCommandExitStatus)
            .map_err(log_scenario_error)?;

        if exit_status != 0 {
            debug!(
                scenario.event = ScenarioEvent::Error.as_str(),
                scenario.error = "Remote command failed with non-zero exit status",
                remote_sudo.exit_status = exit_status as i64
            );
            return Err(RemoteSudoError::RemoteCommandFailedWithStatusCode(
                exit_status,
            ));
        }

        debug!(scenario.event = ScenarioEvent::RemoteSudoCompleted.as_str());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        scenario::{
            remote_sudo::{RemoteSudo, RemoteSudoError},
            variables::Variables,
        },
        session::{Channel, Session, SessionType, Sftp},
        utils::{ArcMutex, Wrap},
    };
    use std::panic;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::TRACE)
            .try_init();
    }

    #[test]
    fn test_execute_success() {
        init_tracing();
        // Given
        struct SuccessChannel;
        impl Channel for SuccessChannel {
            fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
                Ok(())
            }
            fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
                output.push_str("Success output");
                Ok(14)
            }
            fn exit_status(&self) -> Result<i32, ssh2::Error> {
                Ok(0)
            }
        }

        let remote_sudo = RemoteSudo {
            command: "echo success".into(),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(SuccessChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = remote_sudo.execute(&session, &variables, None);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_placeholder_resolution_error() {
        // Given
        let remote_sudo = RemoteSudo {
            command: "{{ missing_var }}".into(),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(TestChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = remote_sudo.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(RemoteSudoError::CannotResolveCommandPlaceholders(_))
        ));
    }

    #[test]
    fn test_execute_channel_exec_failure() {
        // Given
        struct ExecFailChannel;
        impl Channel for ExecFailChannel {
            fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
                Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
            }
            fn read_to_string(&mut self, _output: &mut String) -> Result<usize, ssh2::Error> {
                Ok(0)
            }
            fn exit_status(&self) -> Result<i32, ssh2::Error> {
                Ok(0)
            }
        }

        let remote_sudo = RemoteSudo {
            command: "test".into(),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(ExecFailChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = remote_sudo.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(RemoteSudoError::CannotExecuteRemoteCommand(_))
        ));
    }

    #[test]
    fn test_execute_channel_nonzero_exit_status() {
        // Given
        struct NonZeroExitChannel;
        impl Channel for NonZeroExitChannel {
            fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
                Ok(())
            }
            fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
                output.push_str("error output");
                Ok(12)
            }
            fn exit_status(&self) -> Result<i32, ssh2::Error> {
                Ok(1)
            }
        }

        let remote_sudo = RemoteSudo {
            command: "test".into(),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(NonZeroExitChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = remote_sudo.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(RemoteSudoError::RemoteCommandFailedWithStatusCode(1))
        ));
    }

    #[test]
    fn test_execute_read_output_failure() {
        // Given
        struct ReadFailChannel;
        impl Channel for ReadFailChannel {
            fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
                Ok(())
            }
            fn read_to_string(&mut self, _output: &mut String) -> Result<usize, ssh2::Error> {
                Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
            }
            fn exit_status(&self) -> Result<i32, ssh2::Error> {
                Ok(0)
            }
        }

        let remote_sudo = RemoteSudo {
            command: "test".into(),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(ReadFailChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = remote_sudo.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(RemoteSudoError::CannotReadChannelOutput(_))
        ));
    }

    #[test]
    fn test_execute_exit_status_failure() {
        // Given
        struct ExitStatusFailChannel;
        impl Channel for ExitStatusFailChannel {
            fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
                Ok(())
            }
            fn read_to_string(&mut self, _output: &mut String) -> Result<usize, ssh2::Error> {
                Ok(0)
            }
            fn exit_status(&self) -> Result<i32, ssh2::Error> {
                Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
            }
        }

        let remote_sudo = RemoteSudo {
            command: "test".into(),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(ExitStatusFailChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = remote_sudo.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(RemoteSudoError::CannotObtainRemoteCommandExitStatus(_))
        ));
    }

    #[test]
    fn test_channel_lock_error() {
        // Given
        let remote_sudo = RemoteSudo {
            command: "test".into(),
        };

        let channel_mutex: ArcMutex<TestChannel> = ArcMutex::wrap(TestChannel);
        let channel_mutex_clone = channel_mutex.clone();
        let _ = std::thread::spawn(move || {
            panic::set_hook(Box::new(|_info| {
            }));
            let _ = panic::catch_unwind(|| {
                let _guard = channel_mutex_clone.lock().unwrap();
                panic!("Deliberately poisoning the mutex");
            });
        })
        .join();

        let session = Session {
            inner: SessionType::Test {
                channel: channel_mutex,
                sftp: ArcMutex::wrap(TestSftp),
            },
        };

        let variables = Variables::default();

        // When
        let result = remote_sudo.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(RemoteSudoError::CannotGetALockOnChannel)
        ));
    }

    struct TestWrite;
    impl crate::session::Write for TestWrite {
        fn write_all(&mut self, _buf: &[u8]) -> Result<(), ssh2::Error> {
            Ok(())
        }
    }

    struct TestSftp;
    impl Sftp for TestSftp {
        fn create(
            &self,
            _path: &std::path::Path,
        ) -> Result<Box<dyn crate::session::Write>, ssh2::Error> {
            Ok(Box::new(TestWrite))
        }
    }

    struct TestChannel;
    impl Channel for TestChannel {
        fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
            Ok(())
        }
        fn read_to_string(&mut self, _output: &mut String) -> Result<usize, ssh2::Error> {
            Ok(0)
        }
        fn exit_status(&self) -> Result<i32, ssh2::Error> {
            Ok(0)
        }
    }

    #[test]
    fn test_execute_with_tracker() {
        // Given
        use crate::state::{
            types::{ExecutionState, ExecutionStatus, StepExecState, StepStatus},
            ExecutionStateManager, TaskTracker,
        };
        use std::sync::mpsc;

        struct TrackerChannel;
        impl Channel for TrackerChannel {
            fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> { Ok(()) }
            fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
                output.push_str("tracked output");
                Ok(14)
            }
            fn exit_status(&self) -> Result<i32, ssh2::Error> { Ok(0) }
        }

        let remote_sudo = RemoteSudo { command: "echo ok".into() };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(TrackerChannel),
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
        let tracker = TaskTracker::for_step(&sm, 0);

        // When
        let result = remote_sudo.execute(&session, &variables, Some(&tracker));

        // Then
        assert!(result.is_ok());
        let snapshot = sm.snapshot();
        assert!(snapshot.steps[0].output.contains("tracked output"));
    }

    #[test]
    fn test_execute_channel_session_failure() {
        init_tracing();
        // Given
        let remote_sudo = RemoteSudo {
            command: "echo test".into(),
        };
        let session = Session {
            inner: SessionType::FailSession(ssh2::ErrorCode::Session(libc::EIO)),
        };
        let variables = Variables::default();

        // When
        let result = remote_sudo.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(RemoteSudoError::CannotEstablishSessionChannel(_))
        ));
    }
}
