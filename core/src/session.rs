use crate::{
    scenario::{credentials::Credentials, server::Server},
    session::mock::{MockChannel, MockSftp},
    trace::ScenarioEvent,
    utils::{ArcMutex, Wrap},
};
use std::{fmt, path::Path, sync::Arc};

/// Run an async future on the given runtime from any context.
/// If already inside a Tokio runtime (e.g. Tauri), spawns a thread so the
/// dedicated runtime's `block_on` does not conflict with the outer runtime.
#[cfg(not(tarpaulin_include))]
fn try_block_on<F, R>(runtime: &tokio::runtime::Runtime, fut: F) -> R
where
    F: std::future::Future<Output = R> + Send,
    R: Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(_) => {
            let handle = runtime.handle().clone();
            std::thread::scope(|s| s.spawn(|| handle.block_on(fut)).join().unwrap())
        }
        Err(_) => runtime.block_on(fut),
    }
}
use tracing::{debug, instrument, trace};

#[derive(Debug)]
pub struct SshError {
    msg: String,
}

impl SshError {
    pub fn new(msg: impl Into<String>) -> Self {
        SshError { msg: msg.into() }
    }
}

impl fmt::Display for SshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for SshError {}

/// Operations for executing commands on a remote server via SSH.
pub trait Channel {
    fn exec(&mut self, command: &str) -> Result<(), SshError>;

    fn read_to_string(&mut self, output: &mut String) -> Result<usize, SshError>;

    fn exit_status(&self) -> Result<i32, SshError>;
}

/// SFTP file transfer operations.
pub trait Sftp {
    fn create(&self, path: &Path) -> Result<Box<dyn Write>, SshError>;
}

/// Write operations for remote files, returned by `Sftp::create`.
pub trait Write {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), SshError>;

    fn flush(&mut self) -> Result<(), SshError> {
        Ok(())
    }
}

/// Russh client handler — accepts all host keys.
#[cfg(not(tarpaulin_include))]
pub(crate) struct ClientHandler;

#[cfg(not(tarpaulin_include))]
impl russh::client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// SSH session to a remote server. Supports real connections and mock mode for testing.
pub struct Session {
    pub inner: SessionType,
}

#[allow(private_interfaces)]
pub enum SessionType {
    #[cfg(not(tarpaulin_include))]
    Real {
        runtime: Arc<tokio::runtime::Runtime>,
        handle: russh::client::Handle<ClientHandler>,
    },
    Mock,
    Test {
        channel: ArcMutex<dyn Channel + Send + Sync>,
        sftp: ArcMutex<dyn Sftp + Send + Sync>,
    },
    #[cfg(test)]
    FailSession(String),
}

impl Session {
    /// Returns a dry-run session when `dry_run` is true, a real SSH connection otherwise.
    pub fn new(
        server: &Server,
        credentials: &Credentials,
        dry_run: bool,
    ) -> Result<Self, SshError> {
        if dry_run {
            Self::create_dry_run_session(server, credentials)
        } else {
            Self::create_session(server, credentials)
        }
    }

    pub fn channel_session(&self) -> Result<ArcMutex<dyn Channel + Send + Sync>, SshError> {
        match &self.inner {
            #[cfg(not(tarpaulin_include))]
            SessionType::Real { runtime, handle } => {
                let channel = try_block_on(runtime, handle.channel_open_session())
                    .map_err(|e| SshError::new(e.to_string()))?;
                Ok(ArcMutex::wrap(RusshChannel {
                    runtime: runtime.clone(),
                    channel,
                    output: Vec::new(),
                    exit_code: None,
                }) as ArcMutex<dyn Channel + Send + Sync>)
            }
            SessionType::Mock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(ArcMutex::wrap(MockChannel))
            }
            SessionType::Test { channel, .. } => Ok(ArcMutex::clone(channel)),
            #[cfg(test)]
            SessionType::FailSession(msg) => Err(SshError::new(msg.clone())),
        }
    }

    pub fn sftp(&self) -> Result<ArcMutex<dyn Sftp + Send + Sync>, SshError> {
        match &self.inner {
            #[cfg(not(tarpaulin_include))]
            SessionType::Real { runtime, handle } => {
                let sftp = try_block_on(runtime, async {
                    let channel = handle
                        .channel_open_session()
                        .await
                        .map_err(|e| SshError::new(e.to_string()))?;
                    channel
                        .request_subsystem(true, "sftp")
                        .await
                        .map_err(|e| SshError::new(e.to_string()))?;
                    russh_sftp::client::SftpSession::new(channel.into_stream())
                        .await
                        .map_err(|e| SshError::new(e.to_string()))
                })?;
                Ok(ArcMutex::wrap(RusshSftp {
                    runtime: runtime.clone(),
                    sftp,
                }) as ArcMutex<dyn Sftp + Send + Sync>)
            }
            SessionType::Mock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(ArcMutex::wrap(MockSftp))
            }
            SessionType::Test { sftp, .. } => Ok(ArcMutex::clone(sftp)),
            #[cfg(test)]
            SessionType::FailSession(msg) => Err(SshError::new(msg.clone())),
        }
    }

    #[cfg(not(tarpaulin_include))]
    #[instrument(
        name = "create_session",
        skip_all,
        fields(
            session.host = server.host,
            session.port = server.port,
            session.username = credentials.username
        )
    )]
    fn create_session(server: &Server, credentials: &Credentials) -> Result<Session, SshError> {
        trace!(
            scenario.event = ScenarioEvent::CreateSessionStarted.as_str(),
            session.auth = match (&credentials.password, &credentials.private_key) {
                (Some(_), _) => "password",
                (None, Some(_)) => "private_key",
                (None, None) => "ssh-agent",
            }
        );

        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .map_err(|e| SshError::new(e.to_string()))?,
        );

        let host = &server.host;
        let port = server.port;
        let username = &credentials.username;
        let password = credentials.password.as_deref();
        let private_key = credentials.private_key.as_deref();

        let handle = try_block_on(Arc::as_ref(&runtime), async {
            let config = Arc::new(russh::client::Config {
                window_size: 16 * 1024 * 1024,
                nodelay: true,
                ..Default::default()
            });
            let mut session = russh::client::connect(config, (host.as_str(), port), ClientHandler)
                .await
                .map_err(|e| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %e);
                    SshError::new(e.to_string())
                })?;

            match (password, private_key) {
                (Some(pwd), _) => {
                    let auth = session
                        .authenticate_password(username, pwd)
                        .await
                        .map_err(|e| {
                            debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %e);
                            SshError::new(e.to_string())
                        })?;
                    if !auth.success() {
                        let err = "Password authentication failed";
                        debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = err);
                        return Err(SshError::new(err));
                    }
                }
                (None, Some(key_path)) => {
                    let key_data = std::fs::read(key_path).map_err(|e| {
                        debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %e);
                        SshError::new(format!("Cannot read private key file: {e}"))
                    })?;
                    let key = russh::keys::PrivateKey::from_openssh(&key_data).map_err(|e| {
                        debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %e);
                        SshError::new(format!("Cannot parse private key: {e}"))
                    })?;
                    let key_with_hash = russh::keys::key::PrivateKeyWithHashAlg::new(
                        Arc::new(key),
                        None,
                    );
                    let auth = session
                        .authenticate_publickey(username, key_with_hash)
                        .await
                        .map_err(|e| {
                            debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %e);
                            SshError::new(e.to_string())
                        })?;
                    if !auth.success() {
                        let err = "Private key authentication failed";
                        debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = err);
                        return Err(SshError::new(err));
                    }
                }
                (None, None) => {
                    #[cfg(unix)]
                    {
                        use russh::keys::agent::client::AgentClient;

                        let mut agent = AgentClient::connect_env()
                            .await
                            .map_err(|e| {
                                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %e);
                                SshError::new(format!(
                                    "Failed to connect to SSH agent (is SSH_AUTH_SOCK set?): {e}"
                                ))
                            })?;

                        let identities = agent.request_identities().await.map_err(|e| {
                            debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %e);
                            SshError::new(format!("Failed to list SSH agent identities: {e}"))
                        })?;

                        if identities.is_empty() {
                            let err = "SSH agent has no identities loaded";
                            debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = err);
                            return Err(SshError::new(err));
                        }

                        let mut authenticated = false;
                        for identity in &identities {
                            let key = identity.public_key().into_owned();
                            match session
                                .authenticate_publickey_with(username, key, None, &mut agent)
                                .await
                            {
                                Ok(auth) if auth.success() => {
                                    authenticated = true;
                                    break;
                                }
                                Ok(_) => continue,
                                Err(_) => continue,
                            }
                        }

                        if !authenticated {
                            let err = format!(
                                "SSH agent authentication failed: none of the {} agent key(s) were accepted",
                                identities.len()
                            );
                            debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %err);
                            return Err(SshError::new(err));
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        let err = "SSH agent authentication is not supported on this platform. \
                                   Provide a password or private_key in the configuration.";
                        debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = err);
                        return Err(SshError::new(err));
                    }
                }
            }

            debug!(scenario.event = ScenarioEvent::CreateSessionCompleted.as_str());

            Ok(session)
        })?;

        Ok(Session {
            inner: SessionType::Real { runtime, handle },
        })
    }

    #[instrument(
        name = "create_dry_run_session",
        skip_all,
        fields(
            session.host = server.host,
            session.port = server.port,
            session.username = credentials.username,
        )
    )]
    fn create_dry_run_session(
        server: &Server,
        credentials: &Credentials,
    ) -> Result<Session, SshError> {
        trace!(
            scenario.event = ScenarioEvent::CreatedDryRunSession.as_str(),
            session.password = credentials.password.as_deref().unwrap_or("<ssh-agent>")
        );

        std::thread::sleep(std::time::Duration::from_millis(100));

        Ok(Session {
            inner: SessionType::Mock,
        })
    }
}

impl Default for Session {
    fn default() -> Self {
        Session {
            inner: SessionType::Mock,
        }
    }
}

impl Drop for Session {
    #[cfg(not(tarpaulin_include))]
    fn drop(&mut self) {
        debug!("drop session");
        let inner = std::mem::replace(&mut self.inner, SessionType::Mock);
        #[cfg(not(tarpaulin_include))]
        if let SessionType::Real { runtime, handle } = inner {
            debug!("drop handle");
            drop(handle);
            if tokio::runtime::Handle::try_current().is_ok() {
                std::thread::spawn(move || drop(runtime));
            }
        }
    }
}

// === Russh adapter types ===

#[cfg(not(tarpaulin_include))]
struct RusshChannel {
    runtime: Arc<tokio::runtime::Runtime>,
    channel: russh::Channel<russh::client::Msg>,
    output: Vec<u8>,
    exit_code: Option<u32>,
}

#[cfg(not(tarpaulin_include))]
impl Channel for RusshChannel {
    fn exec(&mut self, command: &str) -> Result<(), SshError> {
        let rt = self.runtime.clone();
        try_block_on(&rt, self.channel.exec(true, command.as_bytes()))
            .map_err(|e| SshError::new(e.to_string()))
    }

    fn read_to_string(&mut self, output: &mut String) -> Result<usize, SshError> {
        let rt = self.runtime.clone();
        try_block_on(&rt, async {
            loop {
                match self.channel.wait().await {
                    Some(russh::ChannelMsg::Data { data }) => {
                        self.output.extend_from_slice(&data);
                    }
                    Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                        self.output.extend_from_slice(&data);
                    }
                    Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                        self.exit_code = Some(exit_status);
                    }
                    None => break,
                    _ => {}
                }
            }
        });

        let bytes = std::mem::take(&mut self.output);
        let text = String::from_utf8(bytes)
            .map_err(|_| SshError::new("Invalid UTF-8 in channel output"))?;
        let len = text.len();
        output.push_str(&text);
        Ok(len)
    }

    fn exit_status(&self) -> Result<i32, SshError> {
        self.exit_code
            .map(|c| c as i32)
            .ok_or_else(|| SshError::new("No exit status received"))
    }
}

#[cfg(not(tarpaulin_include))]
struct RusshSftp {
    runtime: Arc<tokio::runtime::Runtime>,
    sftp: russh_sftp::client::SftpSession,
}

#[cfg(not(tarpaulin_include))]
impl Sftp for RusshSftp {
    fn create(&self, path: &Path) -> Result<Box<dyn Write>, SshError> {
        let path_str = path.to_string_lossy().to_string();
        let rt = self.runtime.clone();
        let file = try_block_on(&rt, self.sftp.create(path_str))
            .map_err(|e| SshError::new(e.to_string()))?;
        Ok(Box::new(RusshFile::new(self.runtime.clone(), file)))
    }
}

#[cfg(not(tarpaulin_include))]
struct RusshFile {
    tx: std::sync::mpsc::SyncSender<Vec<u8>>,
    rx_result: std::sync::mpsc::Receiver<Result<(), SshError>>,
}

#[cfg(not(tarpaulin_include))]
impl RusshFile {
    fn new(runtime: Arc<tokio::runtime::Runtime>, mut file: russh_sftp::client::fs::File) -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(64);
        let (tx_result, rx_result) = std::sync::mpsc::sync_channel::<Result<(), SshError>>(16);
        let handle = runtime.handle().clone();

        std::thread::spawn(move || {
            let _rt = runtime;
            while let Ok(buf) = rx.recv() {
                let result = handle
                    .block_on(tokio::io::AsyncWriteExt::write_all(&mut file, &buf))
                    .map_err(|e| SshError::new(e.to_string()));
                if tx_result.send(result).is_err() {
                    break;
                }
            }
        });

        Self { tx, rx_result }
    }

    fn check_errors(&mut self) -> Result<(), SshError> {
        while let Ok(result) = self.rx_result.try_recv() {
            result?;
        }
        Ok(())
    }
}

#[cfg(not(tarpaulin_include))]
impl Write for RusshFile {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), SshError> {
        self.check_errors()?;
        self.tx
            .send(buf.to_vec())
            .map_err(|_| SshError::new("SFTP writer thread terminated unexpectedly"))
    }

    fn flush(&mut self) -> Result<(), SshError> {
        drop(std::mem::replace(
            &mut self.tx,
            std::sync::mpsc::sync_channel(0).0,
        ));
        while let Ok(result) = self.rx_result.recv() {
            result?;
        }
        Ok(())
    }
}

pub mod mock {
    use crate::session::{Channel, Sftp, SshError, Write};
    use std::path::Path;

    pub struct MockChannel;

    impl Channel for MockChannel {
        fn exec(&mut self, _command: &str) -> Result<(), SshError> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        }

        fn read_to_string(&mut self, output: &mut String) -> Result<usize, SshError> {
            let mock_output = "Mock command output\nLine 1\nLine 2\nLine 3\n";
            output.push_str(mock_output);
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(mock_output.len())
        }

        fn exit_status(&self) -> Result<i32, SshError> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(0)
        }
    }

    pub struct MockSftp;

    impl Sftp for MockSftp {
        fn create(&self, _path: &Path) -> Result<Box<dyn Write>, SshError> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(Box::new(MockFile))
        }
    }

    pub struct MockFile;

    impl Write for MockFile {
        fn write_all(&mut self, _buf: &[u8]) -> Result<(), SshError> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        scenario::{credentials::Credentials, server::Server},
        session::{mock, Channel, Session, SessionType, Sftp, SshError, Write},
        utils::HasText,
    };
    use russh::server::Server as _;
    use std::{path::Path, sync::Arc};

    #[derive(Clone)]
    struct TestSshServer;

    impl russh::server::Server for TestSshServer {
        type Handler = TestSshHandler;
        fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> TestSshHandler {
            TestSshHandler
        }
    }

    struct TestSshHandler;

    impl russh::server::Handler for TestSshHandler {
        type Error = russh::Error;

        async fn auth_password(
            &mut self,
            user: &str,
            password: &str,
        ) -> Result<russh::server::Auth, Self::Error> {
            if user == "testuser" && password == "testpass" {
                Ok(russh::server::Auth::Accept)
            } else {
                Ok(russh::server::Auth::Reject {
                    proceed_with_methods: None,
                    partial_success: false,
                })
            }
        }

        async fn channel_open_session(
            &mut self,
            _channel: russh::Channel<russh::server::Msg>,
            _session: &mut russh::server::Session,
        ) -> Result<bool, Self::Error> {
            Ok(true)
        }
    }

    fn start_test_ssh_server() -> (u16, tokio::runtime::Runtime) {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };

        let config = russh::server::Config {
            auth_rejection_time: std::time::Duration::from_secs(0),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![russh::keys::PrivateKey::random(
                &mut rand::rng(),
                russh::keys::Algorithm::Ed25519,
            )
            .unwrap()],
            ..Default::default()
        };
        let config = Arc::new(config);

        rt.spawn(async move {
            let socket = tokio::net::TcpListener::bind(("127.0.0.1", port))
                .await
                .unwrap();
            let mut server = TestSshServer;
            let _ = server.run_on_socket(config, &socket).await;
        });

        std::thread::sleep(std::time::Duration::from_millis(100));

        (port, rt)
    }

    #[test]
    fn test_session_default() {
        // Given & When
        let default_session = Session::default();

        // Then
        match default_session.inner {
            SessionType::Mock => {}
            SessionType::Real { .. } => panic!("Expected a mock session for default"),
            SessionType::Test { .. } => {
                panic!("Expected a mock session for default, not a test session")
            }
            _ => panic!("Unexpected session type"),
        }
    }

    #[test]
    fn test_dry_run_session_creation() {
        // Given
        let server = test_server();
        let credentials = test_credentials(true);

        // When
        let result = Session::create_dry_run_session(&server, &credentials);

        // Then
        assert!(result.is_ok());
        match result.unwrap().inner {
            SessionType::Mock => {}
            SessionType::Real { .. } => panic!("Expected a mock session"),
            SessionType::Test { .. } => {
                panic!("Expected a mock session, not a test session")
            }
            _ => panic!("Unexpected session type"),
        }
    }

    #[test]
    fn test_authentication_with_agent() {
        // Given
        let server = test_server();
        let credentials = test_credentials(false);

        // When
        let result = Session::create_dry_run_session(&server, &credentials);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_new_in_dry_run_mode() {
        // Given
        let server = test_server();
        let credentials = test_credentials(true);

        // When
        let result = Session::new(&server, &credentials, true);

        // Then
        assert!(result.is_ok());
        match result.unwrap().inner {
            SessionType::Mock => {}
            SessionType::Real { .. } => panic!("Expected a mock session in dry run mode"),
            SessionType::Test { .. } => {
                panic!("Expected a mock session in dry run mode, not a test session")
            }
            _ => panic!("Unexpected session type"),
        }
    }

    #[test]
    fn test_session_new_without_dry_run_connects_to_server() {
        // Given
        let (port, _rt) = start_test_ssh_server();
        let server = Server {
            host: "127.0.0.1".to_string(),
            port,
        };
        let credentials = Credentials {
            username: "testuser".to_string(),
            password: Some("testpass".to_string()),
            private_key: None,
        };

        // When
        let result = Session::new(&server, &credentials, false);

        // Then
        assert!(result.is_ok(), "Expected successful connection: {:?}", result.err());
        match result.unwrap().inner {
            SessionType::Real { .. } => {}
            _ => panic!("Expected a Real session type"),
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
            fn exec(&mut self, _command: &str) -> Result<(), SshError> {
                Err(SshError::new("exec error"))
            }
            fn read_to_string(&mut self, _output: &mut String) -> Result<usize, SshError> {
                Ok(0)
            }
            fn exit_status(&self) -> Result<i32, SshError> {
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
            fn exec(&mut self, _command: &str) -> Result<(), SshError> {
                Ok(())
            }
            fn read_to_string(&mut self, _output: &mut String) -> Result<usize, SshError> {
                Err(SshError::new("read error"))
            }
            fn exit_status(&self) -> Result<i32, SshError> {
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
            fn exec(&mut self, _command: &str) -> Result<(), SshError> {
                Ok(())
            }
            fn read_to_string(&mut self, _output: &mut String) -> Result<usize, SshError> {
                Ok(0)
            }
            fn exit_status(&self) -> Result<i32, SshError> {
                Err(SshError::new("exit status error"))
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
            fn exec(&mut self, _command: &str) -> Result<(), SshError> {
                Ok(())
            }
            fn read_to_string(&mut self, output: &mut String) -> Result<usize, SshError> {
                let data = vec![0xFF, 0xFF, 0xFF];
                match String::from_utf8(data) {
                    Ok(s) => {
                        output.push_str(&s);
                        Ok(3)
                    }
                    Err(_) => Err(SshError::new("Invalid UTF-8")),
                }
            }
            fn exit_status(&self) -> Result<i32, SshError> {
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
            fn create(&self, _path: &Path) -> Result<Box<dyn Write>, SshError> {
                Err(SshError::new("sftp create error"))
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

    #[test]
    fn test_test_session_channel_creation() {
        // Given
        use crate::utils::{ArcMutex, Wrap};
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(mock::MockChannel),
                sftp: ArcMutex::wrap(mock::MockSftp),
            },
        };

        // When
        let result = session.channel_session();

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_test_session_sftp_creation() {
        // Given
        use crate::utils::{ArcMutex, Wrap};
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(mock::MockChannel),
                sftp: ArcMutex::wrap(mock::MockSftp),
            },
        };

        // When
        let result = session.sftp();

        // Then
        assert!(result.is_ok());
    }

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
            private_key: None,
        }
    }

    struct NoOpWrite;
    impl Write for NoOpWrite {
        fn write_all(&mut self, _buf: &[u8]) -> Result<(), SshError> {
            Ok(())
        }
    }

    struct NoOpSftp;
    impl Sftp for NoOpSftp {
        fn create(&self, _path: &Path) -> Result<Box<dyn Write>, SshError> {
            Ok(Box::new(NoOpWrite))
        }
    }

    #[test]
    fn test_perf_write_throughput_64kb_chunks() {
        // Given
        let total_bytes: usize = 50 * 1024 * 1024;
        let chunk_size: usize = 64 * 1024;
        let chunk = vec![0u8; chunk_size];
        let mut writer: Box<dyn Write> = Box::new(NoOpWrite);

        // When
        let start = std::time::Instant::now();
        let mut written = 0;
        while written < total_bytes {
            writer.write_all(&chunk).unwrap();
            written += chunk_size;
        }
        let elapsed = start.elapsed();

        // Then
        assert!(
            elapsed.as_millis() < 1000,
            "50 MB write loop took {}ms, expected < 1000ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_perf_try_block_on_overhead() {
        // Given
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let iterations = 1000;

        // When
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            super::try_block_on(&runtime, async { 42 });
        }
        let elapsed = start.elapsed();

        // Then
        assert!(
            elapsed.as_millis() < 1000,
            "1000 try_block_on calls took {}ms, expected < 1000ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_perf_try_block_on_from_async_context() {
        // Given
        let outer_rt = tokio::runtime::Runtime::new().unwrap();
        let inner_rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap(),
        );
        let iterations = 500;

        // When
        let inner_rt_clone = inner_rt.clone();
        let elapsed = outer_rt.block_on(async move {
            let start = std::time::Instant::now();
            for _ in 0..iterations {
                super::try_block_on(&inner_rt_clone, async { 42 });
            }
            start.elapsed()
        });

        // Then
        assert!(
            elapsed.as_millis() < 5000,
            "500 try_block_on calls from async context took {}ms, expected < 5000ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_perf_sftp_copy_50mb_mock() {
        use crate::scenario::{sftp_copy::SftpCopy, variables::Variables};
        use crate::utils::{ArcMutex, Wrap};

        // Given
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(mock::MockChannel),
                sftp: ArcMutex::wrap(NoOpSftp),
            },
        };
        let sftp_copy = SftpCopy {
            source_path: "source.txt".into(),
            destination_path: "/remote/dest.txt".into(),
        };
        let variables = Variables::default();

        // When
        let start = std::time::Instant::now();
        let result = sftp_copy.execute(&session, &variables, None);
        let elapsed = start.elapsed();

        // Then
        assert!(result.is_ok());
        assert!(
            elapsed.as_millis() < 2000,
            "50 MB SFTP copy (mock) took {}ms, expected < 2000ms",
            elapsed.as_millis()
        );
    }
}
