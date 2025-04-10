use crate::{
    scenario::{errors::SftpCopyError, variables::Variables},
    session::Session,
};
use std::{io::Read, path::Path};
use tracing::{debug, instrument};

#[cfg(not(test))]
use std::fs::File;

#[cfg(test)]
use tests::TestFile as File;

/// Represents an SFTP copy operation from a local file to a remote destination
///
/// This struct holds source and destination paths for transferring a file
/// from the local system to a remote system using SFTP protocol.
#[derive(Debug, Clone)]
pub struct SftpCopy {
    pub(crate) source_path: String,
    pub(crate) destination_path: String,
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

    /// Executes the SFTP copy operation
    ///
    /// # Arguments
    ///
    /// * `session` - The SSH session to use for SFTP operations
    /// * `variables` - Variables to resolve placeholders in paths
    ///
    /// # Returns
    ///
    /// `Ok(())` if the copy completed successfully, otherwise an appropriate `SftpCopyError`
    #[instrument(skip_all, name = "sftp_copy")]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
    ) -> Result<(), SftpCopyError> {
        let resolved_source = variables
            .resolve_placeholders(&self.source_path)
            .map_err(SftpCopyError::CannotResolveSourcePathPlaceholders)
            .map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?;
        let resolved_destination = variables
            .resolve_placeholders(&self.destination_path)
            .map_err(SftpCopyError::CannotResolveDestinationPathPlaceholders)
            .map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?;

        debug!(
            event = "sftp_copy_started",
            source = %resolved_source,
            destination = %resolved_destination
        );

        let mut source_file = File::open(&resolved_source)
            .map_err(SftpCopyError::CannotOpenSourceFile)
            .map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?;

        let sftp = session
            .sftp()
            .map_err(SftpCopyError::CannotOpenChannelAndInitializeSftp)
            .map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?;

        let mut destination_file = sftp
            .lock()
            .map_err(|_| SftpCopyError::CannotGetALockOnSftpChannel)
            .map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?
            .create(Path::new(&resolved_destination))
            .map_err(SftpCopyError::CannotCreateDestinationFile)
            .map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?;

        let total_bytes = source_file
            .metadata()
            .map_err(SftpCopyError::CannotReadSourceFile)
            .map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?
            .len();

        let mut current_bytes = 0u64;
        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = source_file
                .read(&mut buffer)
                .map_err(SftpCopyError::CannotReadSourceFile)
                .map_err(|error| {
                    debug!(event = "error", error = %error);
                    error
                })?;
            if bytes_read == 0 {
                break;
            }

            destination_file
                .write_all(&buffer[..bytes_read])
                .map_err(SftpCopyError::CannotWriteDestinationFile)
                .map_err(|error| {
                    debug!(event = "error", error = %error);
                    error
                })?;

            current_bytes += bytes_read as u64;

            debug!(
                event = "sftp_copy_progress",
                current = current_bytes,
                total = total_bytes
            );
        }

        debug!(event = "sftp_copy_completed", source = %resolved_source, destination = %resolved_destination);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        scenario::variables::Variables,
        session::{Channel, SessionType, Sftp, Write},
        utils::{ArcMutex, Wrap},
    };
    use std::{io, panic};

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

        pub fn open(path: &str) -> io::Result<Self> {
            match path {
                "cannot-open" => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Cannot open source file",
                )),
                "read-error" => Ok(Self::with_read_error()),
                "metadata-error" => Ok(Self::with_metadata_error()),
                _ => Ok(Self::new()),
            }
        }

        pub fn metadata(&self) -> io::Result<TestFileMetadata> {
            match self.metadata_behavior {
                MetadataBehavior::Success => Ok(TestFileMetadata {}),
                MetadataBehavior::ReturnError => {
                    Err(io::Error::new(io::ErrorKind::Other, "Metadata error"))
                }
            }
        }
    }

    impl Read for TestFile {
        fn read(&mut self, _buf: &mut [u8]) -> Result<usize, io::Error> {
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
                    Err(io::Error::new(io::ErrorKind::Other, "Read error"))
                }
            }
        }
    }

    struct MockSuccessfulChannel;
    impl Channel for MockSuccessfulChannel {
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

    fn create_successful_test_session() -> Session {
        struct MockSuccessfulWrite;
        impl Write for MockSuccessfulWrite {
            fn write_all(&mut self, _buf: &[u8]) -> Result<(), ssh2::Error> {
                Ok(())
            }
        }

        struct MockSuccessfulSftp;
        impl Sftp for MockSuccessfulSftp {
            fn create(&self, _path: &std::path::Path) -> Result<Box<dyn Write>, ssh2::Error> {
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
    fn test_accessors() {
        // Given
        let sftp_copy = SftpCopy {
            source_path: "/path/to/source".into(),
            destination_path: "/path/to/destination".into(),
        };

        // When & Then
        assert_eq!(sftp_copy.source_path(), "/path/to/source");
        assert_eq!(sftp_copy.destination_path(), "/path/to/destination");
    }

    #[test]
    fn test_execute_success() {
        // Given
        let sftp_copy = SftpCopy {
            source_path: "source.txt".into(),
            destination_path: "dest.txt".into(),
        };
        let session = create_successful_test_session();
        let variables = Variables::default();

        // When
        let result = sftp_copy.execute(&session, &variables);

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
        let result = sftp_copy.execute(&session, &variables);

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
        let result = sftp_copy.execute(&session, &variables);

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
            fn create(&self, _path: &std::path::Path) -> Result<Box<dyn Write>, ssh2::Error> {
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
                // do nothing
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
        let result = sftp_copy.execute(&session, &variables);

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
            fn create(&self, _path: &std::path::Path) -> Result<Box<dyn Write>, ssh2::Error> {
                Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
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
        let result = sftp_copy.execute(&session, &variables);

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
            fn write_all(&mut self, _buf: &[u8]) -> Result<(), ssh2::Error> {
                Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
            }
        }

        struct WriteFailSftp;
        impl Sftp for WriteFailSftp {
            fn create(&self, _path: &std::path::Path) -> Result<Box<dyn Write>, ssh2::Error> {
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
        let result = sftp_copy.execute(&session, &variables);

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
        let result = sftp_copy.execute(&session, &variables);

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
        let result = sftp_copy.execute(&session, &variables);

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
        let result = sftp_copy.execute(&session, &variables);

        // Then
        assert!(matches!(
            result,
            Err(SftpCopyError::CannotReadSourceFile(_))
        ));
    }
}
