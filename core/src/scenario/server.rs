use crate::config::server::ServerConfig;

/// Target server for scenario execution (host + SSH port).
#[derive(Clone, Debug)]
pub struct Server {
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl From<&ServerConfig> for Server {
    fn from(server_config: &ServerConfig) -> Self {
        Server {
            host: server_config.host.clone(),
            port: server_config.port.unwrap_or(22),
        }
    }
}

impl Server {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::server::ServerConfig, scenario::server::Server};

    #[test]
    fn test_server_from_config_with_port() {
        // Given
        let config = ServerConfig {
            host: "example.org".to_string(),
            port: Some(2222),
        };

        // When
        let server = Server::from(&config);

        // Then
        assert_eq!(server.host(), "example.org");
        assert_eq!(server.port(), 2222);
    }

    #[test]
    fn test_server_from_config_without_port() {
        // Given
        let config = ServerConfig {
            host: "example.org".to_string(),
            port: None,
        };

        // When
        let server = Server::from(&config);

        // Then
        assert_eq!(server.host(), "example.org");
        assert_eq!(server.port(), 22); // Default port
    }

    #[test]
    fn test_server_accessors() {
        // Given
        let server = Server {
            host: "localhost".to_string(),
            port: 8022,
        };

        // When & Then
        assert_eq!(server.host(), "localhost");
        assert_eq!(server.port(), 8022);
    }

    #[test]
    fn test_server_clone() {
        // Given
        let original = Server {
            host: "testserver".to_string(),
            port: 2222,
        };

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(cloned.host(), original.host());
        assert_eq!(cloned.port(), original.port());
    }

    #[test]
    fn test_server_debug() {
        // Given
        let server = Server {
            host: "debug-host".to_string(),
            port: 8022,
        };

        // When
        let debug_string = format!("{:?}", server);

        // Then
        assert!(debug_string.contains("debug-host"));
        assert!(debug_string.contains("8022"));
    }
}
