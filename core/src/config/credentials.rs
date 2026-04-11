use crate::scenario::errors::ScenarioConfigError;
use schemars::JsonSchema;
use serde::Deserialize;

/// Partial credentials supporting inheritance/merging.
#[derive(Deserialize, Clone, Debug, Default, JsonSchema)]
pub struct PartialCredentialsConfig {
    pub username: Option<String>,
    pub password: Option<String>,
    pub private_key: Option<String>,
}

impl PartialCredentialsConfig {
    /// Merges with `other`, where `other` takes precedence.
    pub fn merge(&self, other: &PartialCredentialsConfig) -> PartialCredentialsConfig {
        PartialCredentialsConfig {
            username: other.username.clone().or_else(|| self.username.clone()),
            password: other.password.clone().or_else(|| self.password.clone()),
            private_key: other.private_key.clone().or_else(|| self.private_key.clone()),
        }
    }
}

/// Complete credentials config.
/// Auth priority: password > private_key > SSH agent.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct CredentialsConfig {
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
}

impl TryFrom<PartialCredentialsConfig> for CredentialsConfig {
    type Error = ScenarioConfigError;

    fn try_from(partial: PartialCredentialsConfig) -> Result<Self, Self::Error> {
        Ok(CredentialsConfig {
            username: partial
                .username
                .ok_or(ScenarioConfigError::MissingUsername)?,
            password: partial.password,
            private_key: partial.private_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::credentials::{CredentialsConfig, PartialCredentialsConfig},
        scenario::errors::ScenarioConfigError,
    };
    use toml;

    #[test]
    fn test_credentials_config_default() {
        // Given & When
        let credentials = CredentialsConfig::default();

        // Then
        assert_eq!(credentials.username, "");
        assert!(credentials.password.is_none());
        assert!(credentials.private_key.is_none());
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
            private_key: None,
        };

        let partial2 = PartialCredentialsConfig {
            username: None,
            password: Some("pass2".to_string()),
            private_key: None,
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
            private_key: None,
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
            private_key: None,
        };

        // When
        let result = CredentialsConfig::try_from(partial);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::MissingUsername) => {}
            _ => panic!("Expected MissingUsername error"),
        }
    }

    fn create_credentials_with_password() -> CredentialsConfig {
        CredentialsConfig {
            username: "test_user".to_string(),
            password: Some("test_pass".to_string()),
            private_key: None,
        }
    }

    fn create_credentials_without_password() -> CredentialsConfig {
        CredentialsConfig {
            username: "test_user".to_string(),
            password: None,
            private_key: None,
        }
    }

    #[test]
    fn test_credentials_config_with_private_key() {
        // Given & When
        let credentials = CredentialsConfig {
            username: "test_user".to_string(),
            password: None,
            private_key: Some("/path/to/key".to_string()),
        };

        // Then
        assert_eq!(credentials.username, "test_user");
        assert!(credentials.password.is_none());
        assert_eq!(credentials.private_key, Some("/path/to/key".to_string()));
    }

    #[test]
    fn test_credentials_config_deserialization_with_private_key() {
        // Given
        let toml_str = r#"
            username = "test_user"
            private_key = "/home/user/.ssh/id_ed25519"
        "#;

        // When
        let credentials: CredentialsConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(credentials.username, "test_user");
        assert!(credentials.password.is_none());
        assert_eq!(credentials.private_key, Some("/home/user/.ssh/id_ed25519".to_string()));
    }

    #[test]
    fn test_credentials_config_deserialization_password_takes_no_effect_on_private_key() {
        // Given
        let toml_str = r#"
            username = "test_user"
            password = "test_pass"
            private_key = "/path/to/key"
        "#;

        // When
        let credentials: CredentialsConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(credentials.password, Some("test_pass".to_string()));
        assert_eq!(credentials.private_key, Some("/path/to/key".to_string()));
    }

    #[test]
    fn test_partial_credentials_merge_private_key() {
        // Given
        let partial1 = PartialCredentialsConfig {
            username: Some("user1".to_string()),
            password: None,
            private_key: Some("/key/one".to_string()),
        };

        let partial2 = PartialCredentialsConfig {
            username: None,
            password: None,
            private_key: None,
        };

        // When
        let merged = partial1.merge(&partial2);

        // Then
        assert_eq!(merged.private_key, Some("/key/one".to_string()));
    }

    #[test]
    fn test_partial_credentials_merge_private_key_override() {
        // Given
        let partial1 = PartialCredentialsConfig {
            username: Some("user1".to_string()),
            password: None,
            private_key: Some("/key/one".to_string()),
        };

        let partial2 = PartialCredentialsConfig {
            username: None,
            password: None,
            private_key: Some("/key/two".to_string()),
        };

        // When
        let merged = partial1.merge(&partial2);

        // Then
        assert_eq!(merged.private_key, Some("/key/two".to_string()));
    }

    #[test]
    fn test_partial_to_complete_conversion_with_private_key() {
        // Given
        let partial = PartialCredentialsConfig {
            username: Some("test_user".to_string()),
            password: None,
            private_key: Some("/path/to/key".to_string()),
        };

        // When
        let complete = CredentialsConfig::try_from(partial).unwrap();

        // Then
        assert_eq!(complete.username, "test_user");
        assert!(complete.password.is_none());
        assert_eq!(complete.private_key, Some("/path/to/key".to_string()));
    }
}
