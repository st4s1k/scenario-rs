use crate::scenario::errors::ScenarioConfigError;
use serde::Deserialize;

/// Partial configuration for authentication credentials that supports inheritance.
///
/// This struct defines the optional credentials used to authenticate with a remote server.
/// If this configuration is merged with another, fields in the other take precedence.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::config::credentials::{PartialCredentialsConfig, CredentialsConfig};
///
/// let partial = PartialCredentialsConfig {
///     username: Some("admin".to_string()),
///     password: Some("secure_password".to_string()),
/// };
///
/// let config = CredentialsConfig::try_from(partial).unwrap();
/// assert_eq!(config.username, "admin");
/// assert_eq!(config.password, Some("secure_password".to_string()));
/// ```
#[derive(Deserialize, Clone, Debug, Default)]
pub struct PartialCredentialsConfig {
    /// Optional username to authenticate as on the remote server
    pub username: Option<String>,
    /// Optional password for authentication (if not provided, SSH agent is used)
    pub password: Option<String>,
}

impl PartialCredentialsConfig {
    /// Merges this configuration with another, with the other taking precedence.
    ///
    /// # Arguments
    ///
    /// * `other` - The configuration to merge with this one
    ///
    /// # Returns
    ///
    /// A new configuration that combines both configurations
    pub fn merge(&self, other: &PartialCredentialsConfig) -> PartialCredentialsConfig {
        PartialCredentialsConfig {
            username: other.username.clone().or_else(|| self.username.clone()),
            password: other.password.clone().or_else(|| self.password.clone()),
        }
    }
}

/// Configuration for authentication credentials.
///
/// This struct defines the credentials used to authenticate with a remote server.
/// If password is not provided, SSH agent authentication will be attempted.
///
/// # Examples
///
/// Creating credentials with a password:
///
/// ```
/// use scenario_rs_core::config::credentials::CredentialsConfig;
///
/// let credentials = CredentialsConfig {
///     username: "admin".to_string(),
///     password: Some("secure_password".to_string()),
/// };
///
/// assert_eq!(credentials.username, "admin");
/// assert_eq!(credentials.password, Some("secure_password".to_string()));
/// ```
///
/// Creating credentials for SSH agent authentication:
///
/// ```
/// use scenario_rs_core::config::credentials::CredentialsConfig;
///
/// let credentials = CredentialsConfig {
///     username: "admin".to_string(),
///     password: None,  // Will use SSH agent
/// };
///
/// assert_eq!(credentials.username, "admin");
/// assert!(credentials.password.is_none());
/// ```
///
/// In a TOML configuration file:
/// ```toml
/// [credentials]
/// username = "admin"
/// password = "secure_password"  # Optional, remove to use SSH agent
/// ```
#[derive(Deserialize, Clone, Debug, Default)]
pub struct CredentialsConfig {
    /// The username to authenticate as on the remote server
    pub username: String,
    /// Optional password for authentication (if not provided, SSH agent is used)
    pub password: Option<String>,
}

impl TryFrom<PartialCredentialsConfig> for CredentialsConfig {
    type Error = ScenarioConfigError;

    /// Converts a partial configuration into a complete configuration.
    ///
    /// # Returns
    ///
    /// * `Ok(CredentialsConfig)` - A complete configuration with all required fields
    /// * `Err` - If any required fields are missing
    fn try_from(partial: PartialCredentialsConfig) -> Result<Self, Self::Error> {
        Ok(CredentialsConfig {
            username: partial
                .username
                .ok_or(ScenarioConfigError::MissingUsername)?,
            password: partial.password,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml;

    // Test helpers
    fn create_credentials_with_password() -> CredentialsConfig {
        CredentialsConfig {
            username: "test_user".to_string(),
            password: Some("test_pass".to_string()),
        }
    }

    fn create_credentials_without_password() -> CredentialsConfig {
        CredentialsConfig {
            username: "test_user".to_string(),
            password: None,
        }
    }

    #[test]
    fn test_credentials_config_default() {
        // Given & When
        let credentials = CredentialsConfig::default();

        // Then
        assert_eq!(credentials.username, "");
        assert!(credentials.password.is_none());
    }

    #[test]
    fn test_credentials_config_with_password() {
        // Given
        let credentials = create_credentials_with_password();

        // When & Then
        assert_eq!(credentials.username, "test_user");
        assert_eq!(credentials.password, Some("test_pass".to_string()));
    }

    #[test]
    fn test_credentials_config_without_password() {
        // Given
        let credentials = create_credentials_without_password();

        // When & Then
        assert_eq!(credentials.username, "test_user");
        assert!(credentials.password.is_none());
    }

    #[test]
    fn test_credentials_config_deserialization_with_password() {
        // Given
        let toml_str = r#"
            username = "test_user"
            password = "test_pass"
        "#;

        // When
        let credentials: CredentialsConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(credentials.username, "test_user");
        assert_eq!(credentials.password, Some("test_pass".to_string()));
    }

    #[test]
    fn test_credentials_config_deserialization_without_password() {
        // Given
        let toml_str = r#"
            username = "test_user"
        "#;

        // When
        let credentials: CredentialsConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(credentials.username, "test_user");
        assert!(credentials.password.is_none());
    }

    #[test]
    fn test_credentials_config_clone() {
        // Given
        let original = create_credentials_with_password();

        // When
        let clone = original.clone();

        // Then
        assert_eq!(clone.username, original.username);
        assert_eq!(clone.password, original.password);
    }

    #[test]
    fn test_credentials_config_debug() {
        // Given
        let credentials = create_credentials_with_password();

        // When
        let debug_str = format!("{:?}", credentials);

        // Then
        assert!(debug_str.contains("test_user"));
        assert!(debug_str.contains("test_pass"));
    }

    #[test]
    fn test_partial_credentials_merge() {
        // Given
        let partial1 = PartialCredentialsConfig {
            username: Some("user1".to_string()),
            password: None,
        };

        let partial2 = PartialCredentialsConfig {
            username: None,
            password: Some("pass2".to_string()),
        };

        // When
        let merged = partial1.merge(&partial2);

        // Then
        assert_eq!(merged.username, Some("user1".to_string()));
        assert_eq!(merged.password, Some("pass2".to_string()));
    }

    #[test]
    fn test_partial_to_complete_conversion() {
        // Given
        let partial = PartialCredentialsConfig {
            username: Some("test_user".to_string()),
            password: Some("test_pass".to_string()),
        };

        // When
        let complete = CredentialsConfig::try_from(partial).unwrap();

        // Then
        assert_eq!(complete.username, "test_user");
        assert_eq!(complete.password, Some("test_pass".to_string()));
    }

    #[test]
    fn test_partial_to_complete_missing_username() {
        // Given
        let partial = PartialCredentialsConfig {
            username: None,
            password: Some("test_pass".to_string()),
        };

        // When
        let result = CredentialsConfig::try_from(partial);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::MissingUsername) => {} // expected
            _ => panic!("Expected MissingUsername error"),
        }
    }
}
