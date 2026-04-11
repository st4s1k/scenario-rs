use crate::{
    config::task::{RemoteSudoTaskConfig, SftpCopyTaskConfig},
    scenario::{remote_sudo::RemoteSudo, sftp_copy::SftpCopy},
};

/// A task that can be executed as part of a scenario: either a remote sudo command or an SFTP copy.
#[derive(Debug, Clone)]
pub enum Task {
    RemoteSudo {
        description: String,
        error_message: String,
        remote_sudo: RemoteSudo,
    },
    SftpCopy {
        description: String,
        error_message: String,
        sftp_copy: SftpCopy,
    },
}

impl Task {
    /// Creates a RemoteSudo task from a config entry, using the task name as default description.
    pub fn from_remote_sudo(name: &str, config: &RemoteSudoTaskConfig) -> Self {
        Task::RemoteSudo {
            description: config
                .description
                .clone()
                .unwrap_or_else(|| name.to_string()),
            error_message: config
                .error_message
                .clone()
                .unwrap_or_else(|| "Remote command failed".to_string()),
            remote_sudo: RemoteSudo {
                command: config.command.clone(),
            },
        }
    }

    /// Creates an SftpCopy task from a config entry, using the task name as default description.
    pub fn from_sftp_copy(name: &str, config: &SftpCopyTaskConfig) -> Self {
        Task::SftpCopy {
            description: config
                .description
                .clone()
                .unwrap_or_else(|| name.to_string()),
            error_message: config
                .error_message
                .clone()
                .unwrap_or_else(|| "File transfer failed".to_string()),
            sftp_copy: SftpCopy {
                source_path: config.source.clone(),
                destination_path: config.destination.clone(),
            },
        }
    }

    /// Returns the human-readable description of the task.
    pub fn description(&self) -> &str {
        match self {
            Task::RemoteSudo { description, .. } => description,
            Task::SftpCopy { description, .. } => description,
        }
    }

    /// Returns the error message to display if the task fails.
    pub fn error_message(&self) -> &str {
        match self {
            Task::RemoteSudo { error_message, .. } => error_message,
            Task::SftpCopy { error_message, .. } => error_message,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::task::{RemoteSudoTaskConfig, SftpCopyTaskConfig},
        scenario::task::Task,
    };

    #[test]
    fn test_task_from_remote_sudo_config() {
        // Given
        let config = RemoteSudoTaskConfig {
            command: "echo test".to_string(),
            description: Some("Remote sudo task".to_string()),
            error_message: Some("Remote command failed".to_string()),
        };

        // When
        let task = Task::from_remote_sudo("test_task", &config);

        // Then
        match task {
            Task::RemoteSudo {
                description,
                error_message,
                remote_sudo,
            } => {
                assert_eq!(description, "Remote sudo task");
                assert_eq!(error_message, "Remote command failed");
                assert_eq!(remote_sudo.command(), "echo test");
            }
            _ => panic!("Expected RemoteSudo variant"),
        }
    }

    #[test]
    fn test_task_from_remote_sudo_defaults() {
        // Given
        let config = RemoteSudoTaskConfig {
            command: "echo test".to_string(),
            description: None,
            error_message: None,
        };

        // When
        let task = Task::from_remote_sudo("my_task", &config);

        // Then
        assert_eq!(task.description(), "my_task");
        assert_eq!(task.error_message(), "Remote command failed");
    }

    #[test]
    fn test_task_from_sftp_copy_config() {
        // Given
        let config = SftpCopyTaskConfig {
            source: "/source/path".to_string(),
            destination: "/dest/path".to_string(),
            description: Some("SFTP copy task".to_string()),
            error_message: Some("File transfer failed".to_string()),
        };

        // When
        let task = Task::from_sftp_copy("copy_task", &config);

        // Then
        match task {
            Task::SftpCopy {
                description,
                error_message,
                sftp_copy,
            } => {
                assert_eq!(description, "SFTP copy task");
                assert_eq!(error_message, "File transfer failed");
                assert_eq!(sftp_copy.source_path(), "/source/path");
                assert_eq!(sftp_copy.destination_path(), "/dest/path");
            }
            _ => panic!("Expected SftpCopy variant"),
        }
    }

    #[test]
    fn test_task_from_sftp_copy_defaults() {
        // Given
        let config = SftpCopyTaskConfig {
            source: "/src".to_string(),
            destination: "/dst".to_string(),
            description: None,
            error_message: None,
        };

        // When
        let task = Task::from_sftp_copy("copy_task", &config);

        // Then
        assert_eq!(task.description(), "copy_task");
        assert_eq!(task.error_message(), "File transfer failed");
    }

    #[test]
    fn test_task_clone() {
        // Given
        let config = RemoteSudoTaskConfig {
            command: "echo test".to_string(),
            description: Some("Test".to_string()),
            error_message: Some("Error".to_string()),
        };
        let original = Task::from_remote_sudo("t", &config);

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(original.description(), cloned.description());
        assert_eq!(original.error_message(), cloned.error_message());
    }

    #[test]
    fn test_task_debug() {
        // Given
        let config = RemoteSudoTaskConfig {
            command: "echo test".to_string(),
            description: Some("Remote sudo task".to_string()),
            error_message: None,
        };
        let task = Task::from_remote_sudo("t", &config);

        // When
        let debug_str = format!("{:?}", task);

        // Then
        assert!(debug_str.contains("RemoteSudo"));
        assert!(debug_str.contains("Remote sudo task"));
    }
}
