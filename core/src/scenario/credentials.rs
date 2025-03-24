use crate::config::credentials::CredentialsConfig;

/// Represents authentication credentials used for scenarios.
///
/// This struct stores the username and optional password for authentication
/// purposes within the scenario system.
#[derive(Clone, Debug)]
pub struct Credentials {
    /// The username for authentication.
    pub(crate) username: String,
    /// An optional password for authentication.
    /// May be None if password authentication is not required.
    pub(crate) password: Option<String>,
}

/// Converts a CredentialsConfig into a Credentials struct.
///
/// This implementation allows for seamless creation of Credentials
/// from configuration data.
impl From<&CredentialsConfig> for Credentials {
    fn from(credentials_config: &CredentialsConfig) -> Self {
        Credentials {
            username: credentials_config.username.clone(),
            password: credentials_config.password.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_credentials_directly() {
        // Given & When
        let credentials = Credentials {
            username: "testuser".to_string(),
            password: Some("testpass".to_string()),
        };

        // Then
        assert_eq!(credentials.username, "testuser");
        assert_eq!(credentials.password, Some("testpass".to_string()));
    }

    #[test]
    fn test_create_credentials_without_password() {
        // Given & When
        let credentials = Credentials {
            username: "testuser".to_string(),
            password: None,
        };

        // Then
        assert_eq!(credentials.username, "testuser");
        assert_eq!(credentials.password, None);
    }

    #[test]
    fn test_create_credentials_with_empty_values() {
        // Given & When
        let credentials = Credentials {
            username: "".to_string(),
            password: Some("".to_string()),
        };

        // Then
        assert_eq!(credentials.username, "");
        assert_eq!(credentials.password, Some("".to_string()));
    }

    #[test]
    fn test_from_credentials_config_with_password() {
        // Given
        let config = CredentialsConfig {
            username: "configuser".to_string(),
            password: Some("configpass".to_string()),
        };

        // When
        let credentials = Credentials::from(&config);

        // Then
        assert_eq!(credentials.username, "configuser");
        assert_eq!(credentials.password, Some("configpass".to_string()));
    }

    #[test]
    fn test_from_credentials_config_without_password() {
        // Given
        let config = CredentialsConfig {
            username: "configuser".to_string(),
            password: None,
        };

        // When
        let credentials = Credentials::from(&config);

        // Then
        assert_eq!(credentials.username, "configuser");
        assert_eq!(credentials.password, None);
    }

    #[test]
    fn test_from_credentials_config_with_empty_values() {
        // Given
        let config = CredentialsConfig {
            username: "".to_string(),
            password: Some("".to_string()),
        };

        // When
        let credentials = Credentials::from(&config);

        // Then
        assert_eq!(credentials.username, "");
        assert_eq!(credentials.password, Some("".to_string()));
    }

    #[test]
    fn test_credentials_debug_representation() {
        // Given
        let credentials = Credentials {
            username: "user123".to_string(),
            password: Some("pass123".to_string()),
        };

        // When
        let debug_str = format!("{:?}", credentials);

        // Then
        assert!(debug_str.contains("user123"));
        assert!(debug_str.contains("pass123"));
    }

    #[test]
    fn test_credentials_clone() {
        // Given
        let original = Credentials {
            username: "cloneuser".to_string(),
            password: Some("clonepass".to_string()),
        };

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(original.username, cloned.username);
        assert_eq!(original.password, cloned.password);
    }

    #[test]
    fn test_credentials_with_special_characters() {
        // Given & When
        let credentials = Credentials {
            username: "user@123!#$%".to_string(),
            password: Some("p@ss!#$%^&*()".to_string()),
        };

        // Then
        assert_eq!(credentials.username, "user@123!#$%");
        assert_eq!(credentials.password, Some("p@ss!#$%^&*()".to_string()));
    }

    #[test]
    fn test_credentials_with_very_long_strings() {
        // Given & When
        let long_string = "a".repeat(1000);
        let credentials = Credentials {
            username: long_string.clone(),
            password: Some(long_string.clone()),
        };

        // Then
        assert_eq!(credentials.username.len(), 1000);
        assert_eq!(credentials.password, Some(long_string));
    }
}
