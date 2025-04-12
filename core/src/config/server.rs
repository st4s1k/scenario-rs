use serde::Deserialize;

/// Configuration for a remote server connection.
///
/// This struct defines the connection parameters for the remote server
/// where deployment scenarios will be executed.
///
/// # Examples
///
/// Creating a server configuration with default port (22):
///
/// ```
/// use scenario_rs_core::config::server::ServerConfig;
///
/// let config = ServerConfig {
///     host: "example.com".to_string(),
///     port: None,
/// };
///
/// assert_eq!(config.host, "example.com");
/// assert_eq!(config.port, None);
/// ```
///
/// Creating a server configuration with a custom port:
///
/// ```
/// use scenario_rs_core::config::server::ServerConfig;
///
/// let config = ServerConfig {
///     host: "example.com".to_string(),
///     port: Some(2222),
/// };
///
/// assert_eq!(config.host, "example.com");
/// assert_eq!(config.port, Some(2222));
/// ```
///
/// Deserializing from TOML:
///
/// ```no_run
/// use scenario_rs_core::config::server::ServerConfig;
/// use toml;
///
/// let toml_str = r#"
/// host = "example.com"
/// port = 2222
/// "#;
///
/// let config: ServerConfig = toml::from_str(toml_str).unwrap();
/// assert_eq!(config.host, "example.com");
/// assert_eq!(config.port, Some(2222));
/// ```
#[derive(Deserialize, Clone, Debug, Default)]
pub struct ServerConfig {
    /// The hostname or IP address of the target server
    pub host: String,
    /// The SSH port to connect to (defaults to 22 if not specified)
    pub port: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml;

    // Test helpers
    fn create_test_toml() -> String {
        r#"
            host = "test.example.com"
            port = 2222
        "#
        .to_string()
    }

    #[test]
    fn test_server_config_default() {
        // Given & When
        let config = ServerConfig::default();

        // Then
        assert_eq!(config.host, "");
        assert_eq!(config.port, None);
    }

    #[test]
    fn test_server_config_deserialization_with_port() {
        // Given
        let toml_str = create_test_toml();

        // When
        let config: ServerConfig = toml::from_str(&toml_str).unwrap();

        // Then
        assert_eq!(config.host, "test.example.com");
        assert_eq!(config.port, Some(2222));
    }

    #[test]
    fn test_server_config_deserialization_without_port() {
        // Given
        let toml_str = r#"
            host = "test.example.com"
        "#;

        // When
        let config: ServerConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(config.host, "test.example.com");
        assert_eq!(config.port, None);
    }

    #[test]
    fn test_server_config_clone() {
        // Given
        let original = ServerConfig {
            host: "test.example.com".to_string(),
            port: Some(2222),
        };

        // When
        let clone = original.clone();

        // Then
        assert_eq!(clone.host, original.host);
        assert_eq!(clone.port, original.port);
    }

    #[test]
    fn test_server_config_debug() {
        // Given
        let config = ServerConfig {
            host: "test.example.com".to_string(),
            port: Some(2222),
        };

        // When
        let debug_str = format!("{:?}", config);

        // Then
        assert!(debug_str.contains("test.example.com"));
        assert!(debug_str.contains("2222"));
    }
}
