use crate::config::server::ServerConfig;

/// Represents a target server for scenario execution.
///
/// This struct holds connection details for the remote server where
/// scenario tasks will be executed. It includes the hostname or IP address
/// and the SSH port to connect to.
///
/// # Examples
///
/// Creating a server configuration:
///
/// ```
/// use scenario_rs_core::{
///     config::server::ServerConfig,
///     scenario::server::Server
/// };
///
/// // Create a server from a server config
/// let config = ServerConfig {
///     host: "example.com".to_string(),
///     port: Some(2222),
/// };
/// let server = Server::from(&config);
///
/// // Access server properties through getters
/// assert_eq!(server.host(), "example.com");
/// assert_eq!(server.port(), 2222);
/// ```
///
/// Creating a server with default port:
///
/// ```
/// use scenario_rs_core::{
///     config::server::ServerConfig,
///     scenario::server::Server
/// };
///
/// // Create a server config with just the host (port will default to 22)
/// let config = ServerConfig {
///     host: "example.com".to_string(),
///     port: None,
/// };
///
/// // Convert the config to a Server instance
/// let server = Server::from(&config);
///
/// assert_eq!(server.host(), "example.com");
/// assert_eq!(server.port(), 22); // Default port
/// ```
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

impl Server {
    /// Returns the host of the server.
    ///
    /// This method provides access to the server's hostname or IP address.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the port of the server.
    ///
    /// This method provides access to the server's SSH port.
    pub fn port(&self) -> u16 {
        self.port
    }
}
