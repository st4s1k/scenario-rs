use serde::Deserialize;

/// Configuration for a task in a scenario.
///
/// A task is a single operation that can be performed as part of a scenario,
/// such as executing a command or copying a file.
///
/// # Examples
///
/// Creating a task that executes a command with sudo privileges:
///
/// ```
/// use scenario_rs_core::config::task::{TaskConfig, TaskType};
///
/// let task = TaskConfig {
///     description: "Update package index".to_string(),
///     error_message: "Failed to update package index".to_string(),
///     task_type: TaskType::RemoteSudo {
///         command: "apt-get update".to_string(),
///     },
/// };
///
/// assert_eq!(task.description, "Update package index");
/// ```
///
/// Deserializing from TOML:
///
/// ```no_run
/// use scenario_rs_core::config::task::TaskConfig;
/// use toml;
///
/// let toml_str = r#"
/// description = "Copy configuration file"
/// error_message = "Failed to copy config file"
/// type = "SftpCopy"
/// source_path = "/local/path/config.json"
/// destination_path = "/remote/path/config.json"
/// "#;
///
/// let task: TaskConfig = toml::from_str(toml_str).unwrap();
/// ```
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskConfig {
    /// Human-readable description of what this task does
    pub description: String,
    /// Error message to display if this task fails
    pub error_message: String,
    /// The specific type of task and its associated configuration
    #[serde(flatten)]
    pub task_type: TaskType,
}

/// Defines the different types of tasks that can be performed in a scenario.
///
/// Each variant corresponds to a specific operation with its own configuration parameters.
///
/// # Examples
///
/// Creating a task type for executing a remote command with sudo:
///
/// ```
/// use scenario_rs_core::config::task::TaskType;
///
/// let task_type = TaskType::RemoteSudo {
///     command: "systemctl restart nginx".to_string(),
/// };
/// ```
///
/// Creating a task type for copying a file via SFTP:
///
/// ```
/// use scenario_rs_core::config::task::TaskType;
///
/// let task_type = TaskType::SftpCopy {
///     source_path: "/local/app/config.json".to_string(),
///     destination_path: "/remote/app/config.json".to_string(),
/// };
/// ```
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TaskType {
    /// Execute a command with sudo privileges on a remote system.
    RemoteSudo {
        /// The command to execute
        command: String,
    },
    /// Copy a file from the local system to a remote system using SFTP.
    SftpCopy {
        /// Path to the source file on the local system
        source_path: String,
        /// Path to the destination on the remote system
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
    use super::*;
    use toml;

    // Test helpers
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
}
