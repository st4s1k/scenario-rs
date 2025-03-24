use crate::{
    config::task::{TaskConfig, TaskType},
    scenario::{remote_sudo::RemoteSudo, sftp_copy::SftpCopy},
};

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
    pub fn description(&self) -> &str {
        match self {
            Task::RemoteSudo { description, .. } => description,
            Task::SftpCopy { description, .. } => description,
        }
    }

    pub fn error_message(&self) -> &str {
        match self {
            Task::RemoteSudo { error_message, .. } => error_message,
            Task::SftpCopy { error_message, .. } => error_message,
        }
    }
}
