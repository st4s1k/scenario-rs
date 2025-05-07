use crate::{
    scenario::{errors::RemoteSudoError, variables::Variables},
    session::Session,
};
use tracing::{debug, instrument};

/// Represents a remote command to be executed with sudo privileges
///
/// This struct holds a command string to be executed on a remote session
/// with elevated permissions.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::scenario::remote_sudo::RemoteSudo;
///
/// // Create a simple sudo command
/// let sudo_command = RemoteSudo::new("apt-get update".to_string());
///
/// assert_eq!(sudo_command.command(), "apt-get update");
/// ```
///
/// Using placeholders that will be replaced with variables during execution:
///
/// ```
/// use scenario_rs_core::scenario::remote_sudo::RemoteSudo;
///
/// // Create a command that uses placeholders
/// let sudo_command = RemoteSudo::new("mkdir -p /home/{username}/data".to_string());
/// assert_eq!(sudo_command.command(), "mkdir -p /home/{username}/data");
/// ```
#[derive(Debug, Clone)]
pub struct RemoteSudo {
    pub(crate) command: String,
}

impl RemoteSudo {
    /// Creates a new `RemoteSudo` instance with the given command string
    ///
    /// # Arguments
    /// * `command` - The command string to be executed with sudo privileges
    ///
    /// # Returns
    /// * `RemoteSudo` - A new instance of `RemoteSudo` with the specified command
    /// # Examples
    /// ```
    /// use scenario_rs_core::scenario::remote_sudo::RemoteSudo;
    ///
    /// let sudo_command = RemoteSudo::new("ls -l".to_string());
    ///
    /// assert_eq!(sudo_command.command(), "ls -l");
    /// ```
    pub fn new(command: String) -> Self {
        RemoteSudo { command }
    }

    /// Returns a reference to the command string
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Executes the sudo command on the remote session
    ///
    /// # Arguments
    ///
    /// * `session` - The session to execute the command on
    /// * `variables` - Variables to resolve placeholders in the command
    ///
    /// # Returns
    ///
    /// `Ok(())` if the command executed successfully with exit code 0,
    /// otherwise an appropriate `RemoteSudoError`
    #[instrument(
        name = "remote_sudo",
        skip_all,
        fields(remote_sudo.command)
    )]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
    ) -> Result<(), RemoteSudoError> {
        let command = variables
            .resolve_placeholders(&self.command)
            .map_err(RemoteSudoError::CannotResolveCommandPlaceholders)
            .map_err(|error| {
                debug!(
                    scenario.event = "error",
                    scenario.error = %error,
                    remote_sudo.command = self.command
                );
                error
            })?;

        tracing::Span::current().record("remote_sudo.command", &command);

        debug!(scenario.event = "remote_sudo_started");

        let channel = session
            .channel_session()
            .map_err(RemoteSudoError::CannotEstablishSessionChannel)
            .map_err(|error| {
                debug!(scenario.event = "error", scenario.error = %error);
                error
            })?;

        channel
            .lock()
            .map_err(|_| RemoteSudoError::CannotGetALockOnChannel)
            .map_err(|error| {
                debug!(scenario.event = "error", scenario.error = %error);
                error
            })?
            .exec(&command)
            .map_err(RemoteSudoError::CannotExecuteRemoteCommand)
            .map_err(|error| {
                debug!(scenario.event = "error", scenario.error = %error);
                error
            })?;

        let mut output = String::new();
        channel
            .lock()
            .map_err(|_| RemoteSudoError::CannotGetALockOnChannel)
            .map_err(|error| {
                debug!(scenario.event = "error", scenario.error = %error);
                error
            })?
            .read_to_string(&mut output)
            .map_err(RemoteSudoError::CannotReadChannelOutput)
            .map_err(|error| {
                debug!(scenario.event = "error", scenario.error = %error);
                error
            })?;

        debug!(
            scenario.event = "remote_sudo_output",
            remote_sudo.output = output
        );

        let exit_status = channel
            .lock()
            .map_err(|_| RemoteSudoError::CannotGetALockOnChannel)
            .map_err(|error| {
                debug!(scenario.event = "error", scenario.error = %error);
                error
            })?
            .exit_status()
            .map_err(RemoteSudoError::CannotObtainRemoteCommandExitStatus)
            .map_err(|error| {
                debug!(scenario.event = "error", scenario.error = %error);
                error
            })?;

        if exit_status != 0 {
            debug!(
                scenario.event = "error",
                scenario.error = "Remote command failed with non-zero exit status",
                remote_sudo.exit_status = exit_status as i64
            );
            return Err(RemoteSudoError::RemoteCommandFailedWithStatusCode(
                exit_status,
            ));
        }

        debug!(scenario.event = "remote_sudo_completed");

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

    #[test]
    fn test_execute_success() {
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
        let result = remote_sudo.execute(&session, &variables);

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
        let result = remote_sudo.execute(&session, &variables);

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
        let result = remote_sudo.execute(&session, &variables);

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
        let result = remote_sudo.execute(&session, &variables);

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
        let result = remote_sudo.execute(&session, &variables);

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
        let result = remote_sudo.execute(&session, &variables);

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
                // do nothing
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
        let result = remote_sudo.execute(&session, &variables);

        // Then
        assert!(matches!(
            result,
            Err(RemoteSudoError::CannotGetALockOnChannel)
        ));
    }

    // Test helpers
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
}
