use crate::config::server::ServerConfig;

/// Represents a target server for scenario execution.
///
/// This struct holds connection details for the remote server where
/// scenario tasks will be executed. It includes the hostname or IP address
/// and the SSH port to connect to.
#[derive(Clone, Debug)]
pub struct Server {
    /// The hostname or IP address of the target server
    pub(crate) host: String,
    /// The SSH port to connect to (defaults to 22)
    pub(crate) port: u16,
}

impl From<&ServerConfig> for Server {
    /// Creates a Server instance from a ServerConfig.
    ///
    /// This converts a configuration structure into a runtime server instance,
    /// resolving any unspecified fields with defaults (e.g., using port 22 if not specified).
    fn from(server_config: &ServerConfig) -> Self {
        Server {
            host: server_config.host.clone(),
            port: server_config.port.unwrap_or(22),
        }
    }
}
