use crate::{
    scenario::{errors::SftpCopyError, variables::Variables},
    session::Session,
    state::TaskTracker,
    state::types::TaskProgress,
    trace::ScenarioEvent,
};
use std::{io::Read, path::Path, time::Instant};
use tracing::{debug, instrument, trace};

#[cfg(not(test))]
use std::fs::File;

#[cfg(test)]
use tests::TestFile as File;

/// An SFTP copy operation from a local file to a remote destination.
#[derive(Debug, Clone)]
pub struct SftpCopy {
    pub source_path: String,
    pub destination_path: String,
}

impl SftpCopy {
    /// Returns a reference to the source path
    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    /// Returns a reference to the destination path
    pub fn destination_path(&self) -> &str {
        &self.destination_path
    }

    /// Executes the SFTP copy operation.
    #[instrument(
        name = "sftp_copy",
        skip_all,
        fields(
            sftp_copy.source,
            sftp_copy.destination
        )
    )]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        tracker: Option<&TaskTracker<'_>>,
    ) -> Result<(), SftpCopyError> {
        let resolved_source = variables
            .resolve_placeholders(&self.source_path)
            .map_err(SftpCopyError::CannotResolveSourcePathPlaceholders)
            .map_err(|error| {
                debug!(
                    scenario.event = ScenarioEvent::Error.as_str(),
                    scenario.error = %error,
                    sftp_copy.source = self.source_path,
                    sftp_copy.destination = self.destination_path
                );
                error
            })?;
        let resolved_destination = variables
            .resolve_placeholders(&self.destination_path)
            .map_err(SftpCopyError::CannotResolveDestinationPathPlaceholders)
            .map_err(|error| {
                debug!(
                    scenario.event = ScenarioEvent::Error.as_str(),
                    scenario.error = %error,
                    sftp_copy.source = resolved_source,
                    sftp_copy.destination = self.destination_path
                );
                error
            })?;

        tracing::Span::current()
            .record("sftp_copy.source", resolved_source.as_str())
            .record("sftp_copy.destination", resolved_destination.as_str());

        debug!(scenario.event = ScenarioEvent::SftpCopyStarted.as_str());

        let mut source_file = File::open(&resolved_source)
            .map_err(SftpCopyError::CannotOpenSourceFile)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                error
            })?;

        let sftp = session
            .sftp()
            .map_err(SftpCopyError::CannotOpenChannelAndInitializeSftp)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                error
            })?;

        let mut destination_file = sftp
            .lock()
            .map_err(|_| SftpCopyError::CannotGetALockOnSftpChannel)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                error
            })?
            .create(Path::new(&resolved_destination))
            .map_err(SftpCopyError::CannotCreateDestinationFile)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                error
            })?;

        let total_bytes = source_file
            .metadata()
            .map_err(SftpCopyError::CannotReadSourceFile)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                error
            })?
            .len();

        let mut current_bytes = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        let mut last_progress_emit = Instant::now();
        let throttle_interval = std::time::Duration::from_millis(50);
        loop {
            let bytes_read = source_file
                .read(&mut buffer)
                .map_err(SftpCopyError::CannotReadSourceFile)
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                })?;
            if bytes_read == 0 {
                break;
            }

            destination_file
                .write_all(&buffer[..bytes_read])
                .map_err(SftpCopyError::CannotWriteDestinationFile)
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                })?;

            current_bytes += bytes_read as u64;
            let is_final = current_bytes >= total_bytes;
            let elapsed = last_progress_emit.elapsed() >= throttle_interval;

            if elapsed || is_final {
                trace!(
                    scenario.event = ScenarioEvent::SftpCopyProgress.as_str(),
                    sftp_copy.progress.current = current_bytes,
                    sftp_copy.progress.total = total_bytes,
                );

                if let Some(tracker) = tracker {
                    tracker.update_progress(TaskProgress::SftpCopy {
                        source: resolved_source.clone(),
                        destination: resolved_destination.clone(),
                        bytes_transferred: current_bytes,
                        bytes_total: total_bytes,
                    });
                }

                last_progress_emit = Instant::now();
            }
        }

        destination_file
            .flush()
            .map_err(SftpCopyError::CannotWriteDestinationFile)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                error
            })?;

        debug!(scenario.event = ScenarioEvent::SftpCopyCompleted.as_str());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        scenario::{
            sftp_copy::{SftpCopy, SftpCopyError},
            variables::Variables,
        },
        session::{Channel, Session, SessionType, Sftp, SshError, Write},
        utils::{ArcMutex, Wrap},
    };
    use std::{io::Read, panic};

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
        let sftp_copy = SftpCopy {
            source_path: "source.txt".into(),
            destination_path: "dest.txt".into(),
        };
        let session = create_successful_test_session();
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_source_path_placeholder_resolution_error() {
        // Given
        let sftp_copy = SftpCopy {
            source_path: "{{ missing_var }}".into(),
            destination_path: "/path/to/destination".into(),
        };
        let session = create_successful_test_session();
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotResolveSourcePathPlaceholders(_))
        ));
    }

    #[test]
    fn test_destination_path_placeholder_resolution_error() {
        // Given
        let sftp_copy = SftpCopy {
            source_path: "/path/to/source".into(),
            destination_path: "{{ missing_var }}".into(),
        };
        let session = create_successful_test_session();
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotResolveDestinationPathPlaceholders(_))
        ));
    }

    #[test]
    fn test_sftp_lock_error() {
        // Given
        struct FailLockSftp;
        impl Sftp for FailLockSftp {
            fn create(&self, _path: &std::path::Path) -> Result<Box<dyn Write>, SshError> {
                panic!("Should not be called - lock should fail first")
            }
        }

        let sftp_copy = SftpCopy {
            source_path: "source.txt".into(),
            destination_path: "dest.txt".into(),
        };

        let sftp_mutex: ArcMutex<FailLockSftp> = ArcMutex::wrap(FailLockSftp);

        let sftp_mutex_clone = sftp_mutex.clone();
        let _ = std::thread::spawn(move || {
            panic::set_hook(Box::new(|_info| {
            }));
            let _ = panic::catch_unwind(|| {
                let _guard = sftp_mutex_clone.lock().unwrap();
                panic!("Deliberately poisoning the mutex");
            });
        })
        .join();

        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(MockSuccessfulChannel),
                sftp: sftp_mutex,
            },
        };
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotGetALockOnSftpChannel)
        ));
    }

    #[test]
    fn test_sftp_create_destination_file_error() {
        // Given
        struct FailCreateSftp;
        impl Sftp for FailCreateSftp {
            fn create(&self, _path: &std::path::Path) -> Result<Box<dyn Write>, SshError> {
                Err(SshError::new("sftp create failed"))
            }
        }

        let sftp_copy = SftpCopy {
            source_path: "source.txt".into(),
            destination_path: "dest.txt".into(),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(MockSuccessfulChannel),
                sftp: ArcMutex::wrap(FailCreateSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotCreateDestinationFile(_))
        ));
    }

    #[test]
    fn test_write_destination_file_error() {
        // Given
        struct FailWrite;
        impl Write for FailWrite {
            fn write_all(&mut self, _buf: &[u8]) -> Result<(), SshError> {
                Err(SshError::new("write failed"))
            }
        }

        struct WriteFailSftp;
        impl Sftp for WriteFailSftp {
            fn create(&self, _path: &std::path::Path) -> Result<Box<dyn Write>, SshError> {
                Ok(Box::new(FailWrite))
            }
        }

        let sftp_copy = SftpCopy {
            source_path: "source.txt".into(),
            destination_path: "dest.txt".into(),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(MockSuccessfulChannel),
                sftp: ArcMutex::wrap(WriteFailSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotWriteDestinationFile(_))
        ));
    }

    #[test]
    fn test_cannot_open_source_file() {
        // Given
        let sftp_copy = SftpCopy {
            source_path: "cannot-open".into(),
            destination_path: "dest.txt".into(),
        };
        let session = create_successful_test_session();
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotOpenSourceFile(_))
        ));
    }

    #[test]
    fn test_read_source_file_metadata_error() {
        // Given
        let sftp_copy = SftpCopy {
            source_path: "metadata-error".into(),
            destination_path: "dest.txt".into(),
        };
        let session = create_successful_test_session();
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotReadSourceFile(_))
        ));
    }

    #[test]
    fn test_read_from_source_file_error() {
        // Given
        let sftp_copy = SftpCopy {
            source_path: "read-error".into(),
            destination_path: "dest.txt".into(),
        };
        let session = create_successful_test_session();
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotReadSourceFile(_))
        ));
    }

    enum ReadBehavior {
        Success,
        ReturnError,
    }

    enum MetadataBehavior {
        Success,
        ReturnError,
    }

    pub struct TestFile {
        read_behavior: ReadBehavior,
        metadata_behavior: MetadataBehavior,
    }

    pub struct TestFileMetadata {}

    impl TestFileMetadata {
        pub fn len(&self) -> u64 {
            0
        }
    }

    impl TestFile {
        pub fn new() -> Self {
            Self {
                read_behavior: ReadBehavior::Success,
                metadata_behavior: MetadataBehavior::Success,
            }
        }

        pub fn with_read_error() -> Self {
            Self {
                read_behavior: ReadBehavior::ReturnError,
                metadata_behavior: MetadataBehavior::Success,
            }
        }

        pub fn with_metadata_error() -> Self {
            Self {
                read_behavior: ReadBehavior::Success,
                metadata_behavior: MetadataBehavior::ReturnError,
            }
        }

        pub fn open(path: &str) -> std::io::Result<Self> {
            match path {
                "cannot-open" => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Cannot open source file",
                )),
                "read-error" => Ok(Self::with_read_error()),
                "metadata-error" => Ok(Self::with_metadata_error()),
                _ => Ok(Self::new()),
            }
        }

        pub fn metadata(&self) -> std::io::Result<TestFileMetadata> {
            match self.metadata_behavior {
                MetadataBehavior::Success => Ok(TestFileMetadata {}),
                MetadataBehavior::ReturnError => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Metadata error",
                )),
            }
        }
    }

    impl Read for TestFile {
        fn read(&mut self, _buf: &mut [u8]) -> Result<usize, std::io::Error> {
            match self.read_behavior {
                ReadBehavior::Success => {
                    thread_local! {
                        static ALREADY_READ: std::cell::Cell<bool> = std::cell::Cell::new(false);
                    }
                    ALREADY_READ.with(|already_read| {
                        if already_read.get() {
                            return Ok(0);
                        }
                        already_read.set(true);
                        _buf.fill(0);
                        Ok(_buf.len())
                    })
                }
                ReadBehavior::ReturnError => {
                    Err(std::io::Error::new(std::io::ErrorKind::Other, "Read error"))
                }
            }
        }
    }

    struct MockSuccessfulChannel;
    impl Channel for MockSuccessfulChannel {
        fn exec(&mut self, _command: &str) -> Result<(), SshError> {
            Ok(())
        }
        fn read_to_string(&mut self, _output: &mut String) -> Result<usize, SshError> {
            Ok(0)
        }
        fn exit_status(&self) -> Result<i32, SshError> {
            Ok(0)
        }
    }

    fn create_successful_test_session() -> Session {
        struct MockSuccessfulWrite;
        impl Write for MockSuccessfulWrite {
            fn write_all(&mut self, _buf: &[u8]) -> Result<(), SshError> {
                Ok(())
            }
        }

        struct MockSuccessfulSftp;
        impl Sftp for MockSuccessfulSftp {
            fn create(&self, _path: &std::path::Path) -> Result<Box<dyn Write>, SshError> {
                Ok(Box::new(MockSuccessfulWrite))
            }
        }

        Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(MockSuccessfulChannel),
                sftp: ArcMutex::wrap(MockSuccessfulSftp),
            },
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

        let sftp_copy = SftpCopy {
            source_path: "source.txt".into(),
            destination_path: "dest.txt".into(),
        };
        let session = create_successful_test_session();
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
        let result = sftp_copy.execute(&session, &variables, Some(&tracker));

        // Then
        assert!(result.is_ok());
        let snapshot = sm.snapshot();
        assert!(snapshot.steps[0].progress.is_some());
    }

    #[test]
    fn test_execute_sftp_session_failure() {
        init_tracing();
        // Given
        let sftp_copy = SftpCopy {
            source_path: "source.txt".into(),
            destination_path: "dest.txt".into(),
        };
        let session = Session {
            inner: SessionType::FailSession("sftp session failed".to_string()),
        };
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotOpenChannelAndInitializeSftp(_))
        ));
    }

    #[test]
    fn test_flush_error() {
        init_tracing();

        // Given
        struct FlushFailWrite;
        impl Write for FlushFailWrite {
            fn write_all(&mut self, _buf: &[u8]) -> Result<(), SshError> { Ok(()) }
            fn flush(&mut self) -> Result<(), SshError> {
                Err(SshError::new("flush failed"))
            }
        }

        struct FlushFailSftp;
        impl Sftp for FlushFailSftp {
            fn create(&self, _path: &std::path::Path) -> Result<Box<dyn Write>, SshError> {
                Ok(Box::new(FlushFailWrite))
            }
        }

        let sftp_copy = SftpCopy {
            source_path: "source.txt".into(),
            destination_path: "dest.txt".into(),
        };
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(MockSuccessfulChannel),
                sftp: ArcMutex::wrap(FlushFailSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables, None);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotWriteDestinationFile(_))
        ));
    }
}
