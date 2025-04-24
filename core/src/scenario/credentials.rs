use crate::config::credentials::CredentialsConfig;

/// Represents authentication credentials used for scenarios.
///
/// This struct stores the username and optional password for authentication
/// purposes within the scenario system.
///
/// # Examples
///
/// Creating credentials with a password:
///
/// ```
/// use scenario_rs_core::scenario::credentials::Credentials;
///
/// let credentials = Credentials::new(
///    "testuser".to_string(),
///    Some("testpass".to_string()),
/// );
///
/// assert_eq!(credentials.username(), "testuser");
/// assert_eq!(credentials.password(), Some("testpass"));
/// ```
///
/// Creating credentials without a password (for key-based authentication):
///
/// ```
/// use scenario_rs_core::scenario::credentials::Credentials;
///
/// let credentials = Credentials::new(
///    "testuser".to_string(),
///    None,
/// );
///
/// assert_eq!(credentials.username(), "testuser");
/// assert_eq!(credentials.password(), None);
/// ```
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
///
/// # Examples
///
/// ```
/// use scenario_rs_core::{
///     config::credentials::CredentialsConfig,
///     scenario::credentials::Credentials
/// };
///
/// // Create a credentials config
/// let config = CredentialsConfig {
///     username: "configuser".to_string(),
///     password: Some("configpass".to_string()),
/// };
///
/// // Convert config to credentials
/// let credentials = Credentials::from(&config);
///
/// assert_eq!(credentials.username(), "configuser");
/// assert_eq!(credentials.password(), Some("configpass"));
/// ```
impl From<&CredentialsConfig> for Credentials {
    fn from(credentials_config: &CredentialsConfig) -> Self {
        Credentials {
            username: credentials_config.username.clone(),
            password: credentials_config.password.clone(),
        }
    }
}

impl Credentials {
    /// Creates a new instance of `Credentials` with the given username and optional password.
    pub fn new(username: String, password: Option<String>) -> Self {
        Credentials { username, password }
    }

    /// Returns a reference to the username.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns a reference to the password, if available.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
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
