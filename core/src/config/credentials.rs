use serde::Deserialize;

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
}
