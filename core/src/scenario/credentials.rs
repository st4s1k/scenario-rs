use crate::config::CredentialsConfig;

#[derive(Clone, Debug)]
pub struct Credentials {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
}

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
    fn test_from_credentials_config_with_empty_username() {
        // Given
        let config = CredentialsConfig {
            username: "".to_string(),
            password: Some("configpass".to_string()),
        };

        // When
        let credentials = Credentials::from(&config);

        // Then
        assert_eq!(credentials.username, "");
        assert_eq!(credentials.password, Some("configpass".to_string()));
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
}
