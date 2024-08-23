use crate::{
    config::TaskConfig,
    scenario::{
        errors::TaskError,
        remote_sudo::RemoteSudo,
        sftp_copy::SftpCopy,
        variables::Variables,
    },
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
    fn from(task_config: &TaskConfig) -> Self {
        match task_config {
            TaskConfig::RemoteSudo {
                description,
                error_message,
                remote_sudo: config,
            } => Task::RemoteSudo {
                description: description.clone(),
                error_message: error_message.clone(),
                remote_sudo: RemoteSudo::from(config),
            },
            TaskConfig::SftpCopy {
                description,
                error_message,
                sftp_copy: config,
            } => Task::SftpCopy {
                description: description.clone(),
                error_message: error_message.clone(),
                sftp_copy: SftpCopy::from(config),
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

    pub(crate) fn resolve_placeholders(
        &mut self,
        variables: &Variables,
    ) -> Result<(), TaskError> {
        match self {
            Task::RemoteSudo { remote_sudo, .. } =>
                remote_sudo.resolve_placeholders(variables)
                    .map_err(TaskError::CannotResolveRemoteSudoPlaceholders),
            Task::SftpCopy { sftp_copy, .. } =>
                sftp_copy.resolve_placeholders(variables)
                    .map_err(TaskError::CannotResolveSftpCopyPlaceholders),
        }
    }
}
