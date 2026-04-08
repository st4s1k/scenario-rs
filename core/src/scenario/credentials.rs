use crate::config::credentials::CredentialsConfig;

/// Authentication credentials (username + optional password/key) for scenarios.
#[derive(Clone, Debug)]
pub struct Credentials {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) private_key: Option<String>,
}

impl From<&CredentialsConfig> for Credentials {
    fn from(credentials_config: &CredentialsConfig) -> Self {
        Credentials {
            username: credentials_config.username.clone(),
            password: credentials_config.password.clone(),
            private_key: credentials_config.private_key.clone(),
        }
    }
}

impl Credentials {
    /// Creates a new instance of `Credentials` with the given username and optional password.
    pub fn new(username: String, password: Option<String>) -> Self {
        Credentials {
            username,
            password,
            private_key: None,
        }
    }

    /// Returns a reference to the username.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns a reference to the password, if available.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Returns a reference to the private key path, if available.
    pub fn private_key(&self) -> Option<&str> {
        self.private_key.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::credentials::CredentialsConfig, scenario::credentials::Credentials};

    #[test]
    fn test_create_credentials_with_empty_values() {
        // Given & When
        let credentials = Credentials {
            username: "".to_string(),
            password: Some("".to_string()),
            private_key: None,
        };

        // Then
        assert_eq!(credentials.username, "");
        assert_eq!(credentials.password, Some("".to_string()));
    }

    #[test]
    fn test_from_credentials_config_without_password() {
        // Given
        let config = CredentialsConfig {
            username: "configuser".to_string(),
            password: None,
            private_key: None,
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
            private_key: None,
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
            private_key: None,
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
            private_key: None,
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
            private_key: None,
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
            private_key: None,
        };

        // Then
        assert_eq!(credentials.username.len(), 1000);
        assert_eq!(credentials.password, Some(long_string));
    }

    #[test]
    fn test_credentials_new_with_password() {
        // Given & When
        let credentials = Credentials::new("admin".to_string(), Some("secret".to_string()));

        // Then
        assert_eq!(credentials.username(), "admin");
        assert_eq!(credentials.password(), Some("secret"));
    }

    #[test]
    fn test_credentials_new_without_password() {
        // Given & When
        let credentials = Credentials::new("agent_user".to_string(), None);

        // Then
        assert_eq!(credentials.username(), "agent_user");
        assert_eq!(credentials.password(), None);
        assert_eq!(credentials.private_key(), None);
    }

    #[test]
    fn test_from_credentials_config_with_private_key() {
        // Given
        let config = CredentialsConfig {
            username: "keyuser".to_string(),
            password: None,
            private_key: Some("/path/to/key".to_string()),
        };

        // When
        let credentials = Credentials::from(&config);

        // Then
        assert_eq!(credentials.username(), "keyuser");
        assert_eq!(credentials.password(), None);
        assert_eq!(credentials.private_key(), Some("/path/to/key"));
    }

    #[test]
    fn test_from_credentials_config_with_password_and_private_key() {
        // Given
        let config = CredentialsConfig {
            username: "bothuser".to_string(),
            password: Some("pass".to_string()),
            private_key: Some("/path/to/key".to_string()),
        };

        // When
        let credentials = Credentials::from(&config);

        // Then
        assert_eq!(credentials.password(), Some("pass"));
        assert_eq!(credentials.private_key(), Some("/path/to/key"));
    }

    #[test]
    fn test_credentials_new_has_no_private_key() {
        // Given & When
        let credentials = Credentials::new("user".to_string(), Some("pass".to_string()));

        // Then
        assert_eq!(credentials.private_key(), None);
    }
}
