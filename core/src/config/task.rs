use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for a remote sudo command task.
#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct RemoteSudoTaskConfig {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Configuration for an SFTP file copy task.
#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct SftpCopyTaskConfig {
    pub source: String,
    pub destination: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_sudo_task_config_default() {
        let config = RemoteSudoTaskConfig::default();
        assert_eq!(config.command, "");
        assert!(config.description.is_none());
        assert!(config.error_message.is_none());
    }

    #[test]
    fn test_sftp_copy_task_config_default() {
        let config = SftpCopyTaskConfig::default();
        assert_eq!(config.source, "");
        assert_eq!(config.destination, "");
        assert!(config.description.is_none());
        assert!(config.error_message.is_none());
    }

    #[test]
    fn test_remote_sudo_deserialization() {
        let toml_str = r#"
            command = "apt-get update"
            description = "Update packages"
            error_message = "Update failed"
        "#;
        let config: RemoteSudoTaskConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.command, "apt-get update");
        assert_eq!(config.description.as_deref(), Some("Update packages"));
        assert_eq!(config.error_message.as_deref(), Some("Update failed"));
    }

    #[test]
    fn test_remote_sudo_minimal_deserialization() {
        let toml_str = r#"command = "echo hello""#;
        let config: RemoteSudoTaskConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.command, "echo hello");
        assert!(config.description.is_none());
        assert!(config.error_message.is_none());
    }

    #[test]
    fn test_sftp_copy_deserialization() {
        let toml_str = r#"
            source = "/local/file.txt"
            destination = "/remote/file.txt"
            description = "Deploy config"
            error_message = "Deploy failed"
        "#;
        let config: SftpCopyTaskConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.source, "/local/file.txt");
        assert_eq!(config.destination, "/remote/file.txt");
        assert_eq!(config.description.as_deref(), Some("Deploy config"));
        assert_eq!(config.error_message.as_deref(), Some("Deploy failed"));
    }

    #[test]
    fn test_sftp_copy_minimal_deserialization() {
        let toml_str = r#"
            source = "/local/file.txt"
            destination = "/remote/file.txt"
        "#;
        let config: SftpCopyTaskConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.source, "/local/file.txt");
        assert_eq!(config.destination, "/remote/file.txt");
        assert!(config.description.is_none());
        assert!(config.error_message.is_none());
    }
}
