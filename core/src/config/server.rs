use crate::scenario::errors::ScenarioConfigError;
use serde::Deserialize;

/// Partial configuration for a remote server connection that supports inheritance.
///
/// This struct defines the optional connection parameters for the remote server.
/// If this configuration is merged with another, fields in the other take precedence.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::config::server::{PartialServerConfig, ServerConfig};
///
/// let partial = PartialServerConfig {
///     host: Some("example.com".to_string()),
///     port: Some(2222),
/// };
///
/// let config = ServerConfig::try_from(partial).unwrap();
/// assert_eq!(config.host, "example.com");
/// assert_eq!(config.port, Some(2222));
/// ```
#[derive(Deserialize, Clone, Debug, Default)]
pub struct PartialServerConfig {
    /// Optional hostname or IP address of the target server
    pub host: Option<String>,
    /// Optional SSH port to connect to (defaults to 22 if not specified)
    pub port: Option<u16>,
}

impl PartialServerConfig {
    /// Merges this configuration with another, with the other taking precedence.
    ///
    /// # Arguments
    ///
    /// * `other` - The configuration to merge with this one
    ///
    /// # Returns
    ///
    /// A new configuration that combines both configurations
    pub fn merge(&self, other: &PartialServerConfig) -> PartialServerConfig {
        PartialServerConfig {
            host: other.host.clone().or_else(|| self.host.clone()),
            port: other.port.or(self.port),
        }
    }
}

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
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerConfig {
    /// The hostname or IP address of the target server
    pub host: String,
    /// The SSH port to connect to (defaults to 22 if not specified)
    pub port: Option<u16>,
}

impl TryFrom<PartialServerConfig> for ServerConfig {
    type Error = ScenarioConfigError;

    /// Converts a partial configuration into a complete configuration.
    ///
    /// # Returns
    ///
    /// * `Ok(ServerConfig)` - A complete configuration with all required fields
    /// * `Err` - If any required fields are missing
    fn try_from(partial: PartialServerConfig) -> Result<Self, Self::Error> {
        Ok(ServerConfig {
            host: partial.host.ok_or(ScenarioConfigError::MissingHost)?,
            port: partial.port,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::server::{PartialServerConfig, ServerConfig},
        scenario::errors::ScenarioConfigError,
    };
    use toml;

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

    #[test]
    fn test_partial_server_merge() {
        // Given
        let partial1 = PartialServerConfig {
            host: Some("host1.example.com".to_string()),
            port: None,
        };

        let partial2 = PartialServerConfig {
            host: None,
            port: Some(2222),
        };

        // When
        let merged = partial1.merge(&partial2);

        // Then
        assert_eq!(merged.host, Some("host1.example.com".to_string()));
        assert_eq!(merged.port, Some(2222));
    }

    #[test]
    fn test_partial_to_complete_conversion() {
        // Given
        let partial = PartialServerConfig {
            host: Some("test.example.com".to_string()),
            port: Some(2222),
        };

        // When
        let complete = ServerConfig::try_from(partial).unwrap();

        // Then
        assert_eq!(complete.host, "test.example.com");
        assert_eq!(complete.port, Some(2222));
    }

    #[test]
    fn test_partial_to_complete_missing_host() {
        // Given
        let partial = PartialServerConfig {
            host: None,
            port: Some(2222),
        };

        // When
        let result = ServerConfig::try_from(partial);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::MissingHost) => {} // expected
            _ => panic!("Expected MissingHost error"),
        }
    }

    // Test helpers
    fn create_test_toml() -> String {
        r#"
            host = "test.example.com"
            port = 2222
        "#
        .to_string()
    }
}
