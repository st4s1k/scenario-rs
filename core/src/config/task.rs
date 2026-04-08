use serde::Deserialize;

/// A single task operation in a scenario (command execution or file copy).
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskConfig {
    pub description: String,
    pub error_message: String,
    #[serde(flatten)]
    pub task_type: TaskType,
}

/// The different task operations available.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TaskType {
    RemoteSudo {
        command: String,
    },
    SftpCopy {
        source_path: String,
        destination_path: String,
    },
}

impl Default for TaskType {
    fn default() -> Self {
        TaskType::RemoteSudo {
            command: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::task::{TaskConfig, TaskType};
    use toml;

    #[test]
    fn test_task_type_default_is_remote_sudo() {
        // Given & When
        let task_type = TaskType::default();

        // Then
        assert_eq!(
            task_type,
            TaskType::RemoteSudo {
                command: String::new()
            }
        );
    }

    #[test]
    fn test_task_config_default() {
        // Given & When
        let config = TaskConfig::default();

        // Then
        assert_eq!(config.description, "");
        assert_eq!(config.error_message, "");
        assert_eq!(
            config.task_type,
            TaskType::RemoteSudo {
                command: String::new()
            }
        );
    }

    #[test]
    fn test_task_config_remote_sudo_deserialization() {
        // Given
        let toml_str = create_remote_sudo_toml();

        // When
        let task: TaskConfig = toml::from_str(&toml_str).unwrap();

        // Then
        assert_eq!(task.description, "Update system packages");
        assert_eq!(task.error_message, "Failed to update system packages");

        match task.task_type {
            TaskType::RemoteSudo { command } => {
                assert_eq!(command, "apt-get update && apt-get upgrade -y");
            }
            _ => panic!("Expected RemoteSudo task type"),
        }
    }

    #[test]
    fn test_task_config_sftp_copy_deserialization() {
        // Given
        let toml_str = create_sftp_copy_toml();

        // When
        let task: TaskConfig = toml::from_str(&toml_str).unwrap();

        // Then
        assert_eq!(task.description, "Deploy configuration file");
        assert_eq!(task.error_message, "Failed to deploy configuration file");

        match task.task_type {
            TaskType::SftpCopy {
                source_path,
                destination_path,
            } => {
                assert_eq!(source_path, "/local/config.json");
                assert_eq!(destination_path, "/remote/config.json");
            }
            _ => panic!("Expected SftpCopy task type"),
        }
    }

    #[test]
    fn test_task_config_with_empty_fields() {
        // Given
        let toml_str = r#"
            description = ""
            error_message = ""
            type = "RemoteSudo"
            command = ""
        "#;

        // When
        let task: TaskConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(task.description, "");
        assert_eq!(task.error_message, "");

        match task.task_type {
            TaskType::RemoteSudo { command } => {
                assert_eq!(command, "");
            }
            _ => panic!("Expected RemoteSudo task type"),
        }
    }

    #[test]
    fn test_task_type_discriminant_sensitivity() {
        // Given
        let valid_toml = r#"
            description = "Test task"
            error_message = "Test error"
            type = "RemoteSudo"
            command = "echo test"
        "#;

        let invalid_case_toml = r#"
            description = "Test task"
            error_message = "Test error"
            type = "remotesudo"
            command = "echo test"
        "#;

        // When & Then
        assert!(toml::from_str::<TaskConfig>(valid_toml).is_ok());
        assert!(toml::from_str::<TaskConfig>(invalid_case_toml).is_err());
    }

    #[test]
    fn test_task_type_missing_fields() {
        // Given
        let missing_command_toml = r#"
            description = "Test task"
            error_message = "Test error"
            type = "RemoteSudo"
        "#;

        // When
        let result = toml::from_str::<TaskConfig>(missing_command_toml);

        // Then
        assert!(result.is_err());
    }

    fn create_remote_sudo_toml() -> String {
        r#"
            description = "Update system packages"
            error_message = "Failed to update system packages"
            type = "RemoteSudo"
            command = "apt-get update && apt-get upgrade -y"
        "#
        .to_string()
    }

    fn create_sftp_copy_toml() -> String {
        r#"
            description = "Deploy configuration file"
            error_message = "Failed to deploy configuration file"
            type = "SftpCopy"
            source_path = "/local/config.json"
            destination_path = "/remote/config.json"
        "#
        .to_string()
    }
}
