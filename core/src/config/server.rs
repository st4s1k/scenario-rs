use serde::Deserialize;

/// Configuration for a remote server connection.
///
/// This struct defines the connection parameters for the remote server
/// where deployment scenarios will be executed.
#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    /// The hostname or IP address of the target server
    pub host: String,
    /// The SSH port to connect to (defaults to 22 if not specified)
    pub port: Option<u16>,
}
