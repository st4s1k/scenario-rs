use crate::{
    scenario::{credentials::Credentials, server::Server},
    session::mock::{MockChannel, MockSftp},
};
use std::{
    net::TcpStream,
    path::Path,
    sync::{Arc, Mutex},
};
use tracing::debug;

/// Defines operations for executing commands on a remote server via SSH.
pub trait Channel {
    /// Executes a command on the remote system.
    ///
    /// # Arguments
    /// * `command` - The command to execute on the remote system.
    ///
    /// # Returns
    /// * `Ok(())` if the command was successfully initiated
    /// * `Err` if there was an error executing the command
    fn exec(&mut self, command: &str) -> Result<(), ssh2::Error>;

    /// Reads the output of a command into a string.
    ///
    /// # Arguments
    /// * `output` - String buffer to append the command output to
    ///
    /// # Returns
    /// * `Ok(usize)` with the number of bytes read if successful
    /// * `Err` if there was an error reading the command output
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error>;

    /// Gets the exit status of the command.
    ///
    /// # Returns
    /// * `Ok(i32)` with the exit code of the command
    /// * `Err` if there was an error retrieving the exit status
    fn exit_status(&self) -> Result<i32, ssh2::Error>;
}

/// Defines operations for SFTP file transfer.
pub trait Sftp {
    /// Creates a new file at the specified path for writing.
    ///
    /// # Arguments
    /// * `path` - Path where the file should be created
    ///
    /// # Returns
    /// * `Ok(Box<dyn Write>)` with a writer for the created file
    /// * `Err` if there was an error creating the file
    fn create(&self, path: &Path) -> Result<Box<dyn Write>, ssh2::Error>;
}

/// Defines writing operations for remote files.
pub trait Write {
    /// Writes all bytes from a buffer to the remote file.
    ///
    /// # Arguments
    /// * `buf` - The data to write
    ///
    /// # Returns
    /// * `Ok(())` if all bytes were successfully written
    /// * `Err` if there was an error writing the data
    fn write_all(&mut self, buf: &[u8]) -> Result<(), ssh2::Error>;
}

/// Represents an SSH session to a remote server.
///
/// The session can be either a real SSH connection or a mock session
/// depending on the build configuration and initialization method.
pub struct Session {
    pub(crate) inner: SessionType,
}

/// The internal type of session (real or mock)
pub(crate) enum SessionType {
    Real(ssh2::Session),
    Mock,
    #[cfg(test)]
    Test {
        channel: Arc<Mutex<dyn Channel + Send + Sync>>,
        sftp: Arc<Mutex<dyn Sftp + Send + Sync>>,
    },
}

impl Session {
    /// Creates a new session to the specified server using the provided credentials.
    ///
    /// In debug builds, this returns a mock session. In release builds, it creates
    /// a real SSH connection to the server.
    ///
    /// # Arguments
    /// * `server` - The server connection information
    /// * `credentials` - The authentication credentials
    ///
    /// # Returns
    /// * `Ok(Session)` if the session was created successfully
    /// * `Err` if there was an error establishing the session
    pub fn new(server: &Server, credentials: &Credentials) -> Result<Self, ssh2::Error> {
        if cfg!(debug_assertions) {
            Self::create_mock_session(server, credentials)
        } else {
            Self::create_session(server, credentials)
        }
    }

    /// Creates a command channel for executing remote commands.
    ///
    /// # Returns
    /// * `Ok(Box<dyn Channel>)` with a channel for executing commands
    /// * `Err` if there was an error creating the channel
    pub fn channel_session(&self) -> Result<Arc<Mutex<dyn Channel + Send + Sync>>, ssh2::Error> {
        match &self.inner {
            SessionType::Real(real_session) => real_session
                .channel_session()
                .map(|ch| Arc::new(Mutex::new(ch)) as Arc<Mutex<dyn Channel + Send + Sync>>),
            SessionType::Mock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(Arc::new(Mutex::new(MockChannel)))
            }
            #[cfg(test)]
            SessionType::Test { channel, .. } => Ok(Arc::clone(channel)),
        }
    }

    /// Creates an SFTP session for file transfer operations.
    ///
    /// # Returns
    /// * `Ok(Box<dyn Sftp>)` with an SFTP session
    /// * `Err` if there was an error creating the SFTP session
    pub fn sftp(&self) -> Result<Arc<Mutex<dyn Sftp + Send + Sync>>, ssh2::Error> {
        match &self.inner {
            SessionType::Real(real_session) => real_session
                .sftp()
                .map(|sftp| Arc::new(Mutex::new(sftp)) as Arc<Mutex<dyn Sftp + Send + Sync>>),
            SessionType::Mock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(Arc::new(Mutex::new(MockSftp)))
            }
            #[cfg(test)]
            SessionType::Test { sftp, .. } => Ok(Arc::clone(sftp)),
        }
    }

    /// Creates an SSH session to the specified server.
    ///
    /// # Arguments
    /// * `server` - The server connection information
    /// * `credentials` - The authentication credentials
    ///
    /// # Returns
    /// * `Ok(Session)` if the connection was established successfully
    /// * `Err` if there was an error connecting or authenticating
    fn create_session(server: &Server, credentials: &Credentials) -> Result<Session, ssh2::Error> {
        let host = &server.host;
        let port = &server.port;

        debug!(
            event = "create_session_started",
            host = host,
            port = port,
            username = credentials.username,
            password = credentials.password.as_deref().unwrap_or("<ssh-agent>")
        );

        let tcp = TcpStream::connect(&format!("{host}:{port}")).map_err(|error| {
            debug!(event = "error", error = %error);
            ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO))
        })?;

        let mut real_session = ssh2::Session::new().map_err(|error| {
            debug!(event = "error", error = %error);
            error
        })?;
        real_session.set_tcp_stream(tcp);
        real_session.handshake().map_err(|error| {
            debug!(event = "error", error = %error);
            error
        })?;

        let username = &credentials.username;
        let password = &credentials.password.as_deref();

        match password {
            Some(pwd) => real_session
                .userauth_password(username, pwd)
                .map_err(|error| {
                    debug!(event = "error", error = %error);
                    error
                })?,
            None => real_session.userauth_agent(username).map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?,
        }

        debug!(event = "create_session_completed");

        Ok(Session {
            inner: SessionType::Real(real_session),
        })
    }

    /// Creates a mock SSH session for testing or debugging.
    ///
    /// This doesn't actually connect to a server but simulates the behavior
    /// with delays and mock responses.
    ///
    /// # Arguments
    /// * `server` - The server connection information (for display only)
    /// * `credentials` - The authentication credentials (for display only)
    ///
    /// # Returns
    /// * `Ok(Session)` with a mock session
    fn create_mock_session(
        server: &Server,
        credentials: &Credentials,
    ) -> Result<Session, ssh2::Error> {
        debug!(
            event = "created_mock_session",
            host = server.host,
            port = server.port,
            username = credentials.username,
            password = credentials.password.as_deref().unwrap_or("<ssh-agent>")
        );

        std::thread::sleep(std::time::Duration::from_millis(100));

        Ok(Session {
            inner: SessionType::Mock,
        })
    }
}

impl Default for Session {
    /// Creates a default session, which is a mock session.
    fn default() -> Self {
        Session {
            inner: SessionType::Mock,
        }
    }
}

impl Channel for ssh2::Channel {
    fn exec(&mut self, command: &str) -> Result<(), ssh2::Error> {
        self.exec(command)
    }

    fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
        use std::io::Read;
        let mut buf = Vec::new();
        match self.read_to_end(&mut buf) {
            Ok(size) => {
                if let Ok(s) = String::from_utf8(buf) {
                    output.push_str(&s);
                    Ok(size)
                } else {
                    Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
                }
            }
            Err(_) => Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO))),
        }
    }

    fn exit_status(&self) -> Result<i32, ssh2::Error> {
        self.exit_status()
    }
}

impl Sftp for ssh2::Sftp {
    fn create(&self, path: &Path) -> Result<Box<dyn Write>, ssh2::Error> {
        self.create(path)
            .map(|file| Box::new(file) as Box<dyn Write>)
    }
}

impl Write for ssh2::File {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), ssh2::Error> {
        std::io::Write::write_all(self, buf)
            .map_err(|_| ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
    }
}

/// Provides mock implementations for SSH components to use in tests and debugging.
pub mod mock {
    use super::*;

    /// Mock implementation of the `Channel` trait.
    pub struct MockChannel;

    impl Channel for MockChannel {
        fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        }

        fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
            let mock_output = "Mock command output\nLine 1\nLine 2\nLine 3\n";
            output.push_str(mock_output);
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(mock_output.len())
        }

        fn exit_status(&self) -> Result<i32, ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(0)
        }
    }

    /// Mock implementation of the `Sftp` trait.
    pub struct MockSftp;

    impl Sftp for MockSftp {
        fn create(&self, _path: &Path) -> Result<Box<dyn Write>, ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(Box::new(MockFile))
        }
    }

    /// Mock implementation of the `Write` trait.
    pub struct MockFile;

    impl Write for MockFile {
        fn write_all(&mut self, _buf: &[u8]) -> Result<(), ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{credentials::Credentials, server::Server, utils::HasText};
    use std::path::Path;

    // Test fixtures
    fn test_server() -> Server {
        Server {
            host: "test.example.com".to_string(),
            port: 22,
        }
    }

    fn test_credentials(with_password: bool) -> Credentials {
        Credentials {
            username: "testuser".to_string(),
            password: if with_password {
                Some("testpass".to_string())
            } else {
                None
            },
        }
    }

    #[test]
    fn test_session_default() {
        // Given & When
        let default_session = Session::default();

        // Then
        match default_session.inner {
            SessionType::Mock => {} // Expected
            SessionType::Real(_) => panic!("Expected a mock session for default"),
            SessionType::Test { .. } => {
                panic!("Expected a mock session for default, not a test session")
            }
        }
    }

    #[test]
    fn test_mock_session_creation() {
        // Given
        let server = test_server();
        let credentials = test_credentials(true);

        // When
        let result = Session::create_mock_session(&server, &credentials);

        // Then
        assert!(result.is_ok());
        match result.unwrap().inner {
            SessionType::Mock => {} // Expected
            SessionType::Real(_) => panic!("Expected a mock session"),
            SessionType::Test { .. } => {
                panic!("Expected a mock session, not a test session")
            }
        }
    }

    #[test]
    fn test_authentication_with_agent() {
        // Given
        let server = test_server();
        let credentials = test_credentials(false);

        // When
        let result = Session::create_mock_session(&server, &credentials);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_new_in_debug_mode() {
        // Given
        let server = test_server();
        let credentials = test_credentials(true);

        // When
        let result = Session::new(&server, &credentials);

        // Then
        assert!(result.is_ok());
        if cfg!(debug_assertions) {
            match result.unwrap().inner {
                SessionType::Mock => {} // Expected in debug mode
                SessionType::Real(_) => panic!("Expected a mock session in debug mode"),
                SessionType::Test { .. } => {
                    panic!("Expected a mock session in debug mode, not a test session")
                }
            }
        }
    }

    #[test]
    fn test_mock_session_channel_creation() {
        // Given
        let session = Session::default();

        // When
        let result = session.channel_session();

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_session_sftp_creation() {
        // Given
        let session = Session::default();

        // When
        let result = session.sftp();

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_channel_exec() {
        // Given
        let mut channel = mock::MockChannel;

        // When
        let result = channel.exec("test command");

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_channel_exec_error() {
        // Given
        struct ErrorExecChannel;
        impl Channel for ErrorExecChannel {
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
        let mut channel = ErrorExecChannel;

        // When
        let result = channel.exec("test command");

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_channel_read() {
        // Given
        let mut channel = mock::MockChannel;
        let mut output = String::new();

        // When
        let result = channel.read_to_string(&mut output);

        // Then
        assert!(result.is_ok());
        assert!(output.has_text());
        assert_eq!(output, "Mock command output\nLine 1\nLine 2\nLine 3\n");
    }

    #[test]
    fn test_mock_channel_exit_status() {
        // Given
        let channel = mock::MockChannel;

        // When
        let result = channel.exit_status();

        // Then
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_channel_read_error() {
        // Given
        struct ErrorChannel;
        impl Channel for ErrorChannel {
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
        let mut channel = ErrorChannel;
        let mut output = String::new();

        // When
        let result = channel.read_to_string(&mut output);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_channel_exit_status_error() {
        // Given
        struct ExitStatusErrorChannel;
        impl Channel for ExitStatusErrorChannel {
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
        let channel = ExitStatusErrorChannel;

        // When
        let result = channel.exit_status();

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_channel_read_string_utf8_error() {
        // Given
        struct Utf8ErrorChannel;
        impl Channel for Utf8ErrorChannel {
            fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
                Ok(())
            }
            fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
                // Simulate the situation where we read data but it's not valid UTF-8
                let data = vec![0xFF, 0xFF, 0xFF]; // Invalid UTF-8 bytes
                match String::from_utf8(data) {
                    Ok(s) => {
                        output.push_str(&s);
                        Ok(3)
                    }
                    Err(_) => Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO))),
                }
            }
            fn exit_status(&self) -> Result<i32, ssh2::Error> {
                Ok(0)
            }
        }

        let mut channel = Utf8ErrorChannel;
        let mut output = String::new();

        // When
        let result = channel.read_to_string(&mut output);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_sftp_create() {
        // Given
        let sftp = mock::MockSftp;
        let path = Path::new("/tmp/test.txt");

        // When
        let result = sftp.create(path);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_sftp_create_error() {
        // Given
        struct ErrorSftp;
        impl Sftp for ErrorSftp {
            fn create(&self, _path: &Path) -> Result<Box<dyn Write>, ssh2::Error> {
                Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
            }
        }
        let sftp = ErrorSftp;
        let path = Path::new("/tmp/test.txt");

        // When
        let result = sftp.create(path);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_file_write() {
        // Given
        let mut file = mock::MockFile;
        let data = b"test data";

        // When
        let result = file.write_all(data);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_handles_connection_failures() {
        // Given
        let invalid_server = Server {
            host: "non.existent.host".to_string(),
            port: 0,
        };
        let credentials = test_credentials(false);

        // When
        let result = Session::create_session(&invalid_server, &credentials);

        // Then
        assert!(
            result.is_err(),
            "Expected an error when connecting to invalid server"
        );
    }
}
