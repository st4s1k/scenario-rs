use crate::{
    config::task::{TaskConfig, TaskType},
    scenario::{remote_sudo::RemoteSudo, sftp_copy::SftpCopy},
};

/// Represents a task that can be executed as part of a scenario.
///
/// Tasks are the building blocks of scenarios and can perform various operations
/// such as executing remote commands with sudo privileges or transferring files 
/// via SFTP.
#[derive(Debug, Clone)]
pub enum Task {
    /// A task that executes a command with sudo privileges on a remote server.
    RemoteSudo {
        /// Human-readable description of what this task does
        description: String,
        /// Error message to display if this task fails
        error_message: String,
        /// The remote sudo command execution details
        remote_sudo: RemoteSudo,
    },
    /// A task that copies a file from the local system to a remote server via SFTP.
    SftpCopy {
        /// Human-readable description of what this task does
        description: String,
        /// Error message to display if this task fails
        error_message: String,
        /// The SFTP copy operation details
        sftp_copy: SftpCopy,
    },
}

impl From<&TaskConfig> for Task {
    fn from(config: &TaskConfig) -> Self {
        match &config.task_type {
            TaskType::RemoteSudo { command } => Task::RemoteSudo {
                description: config.description.clone(),
                error_message: config.error_message.clone(),
                remote_sudo: RemoteSudo {
                    command: command.clone(),
                },
            },
            TaskType::SftpCopy {
                source_path,
                destination_path,
            } => Task::SftpCopy {
                description: config.description.clone(),
                error_message: config.error_message.clone(),
                sftp_copy: SftpCopy {
                    source_path: source_path.clone(),
                    destination_path: destination_path.clone(),
                },
            },
        }
    }
}

impl Task {
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
