use crate::{
    config::TaskConfig,
    scenario::{
        errors::TaskError,
        remote_sudo::RemoteSudo,
        sftp_copy::SftpCopy,
        variables::Variables,
    },
};

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

impl TryFrom<(&TaskConfig, &Variables)> for Task {
    type Error = TaskError;

    fn try_from((task_config, variables): (&TaskConfig, &Variables)) -> Result<Self, Self::Error> {
        let task = match task_config {
            TaskConfig::RemoteSudo {
                description,
                error_message,
                remote_sudo: config,
            } => Task::RemoteSudo {
                description: description.clone(),
                error_message: error_message.clone(),
                remote_sudo: RemoteSudo::try_from((config, variables))
                    .map_err(TaskError::CannotCreateRemoteSudoTaskFromConfig)?,
            },
            TaskConfig::SftpCopy {
                description,
                error_message,
                sftp_copy: config,
            } => Task::SftpCopy {
                description: description.clone(),
                error_message: error_message.clone(),
                sftp_copy: SftpCopy::try_from((config, variables))
                    .map_err(TaskError::CannotCreateSftpCopyTaskFromConfig)?,
            },
        };

        Ok(task)
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
