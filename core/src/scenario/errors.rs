//! Error types for the scenario system.
//!
//! This module defines all the error types that can occur during scenario
//! configuration loading, validation, and execution.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScenarioConfigError {
    #[error("Cannot open config file:\n{0}")]
    CannotOpenConfig(#[source] std::io::Error),

    #[error("Cannot read config file:\n{0}")]
    CannotReadConfig(#[source] toml::de::Error),

    #[error("Circular dependency detected:\n{0}")]
    CircularDependency(String),

    #[error("Missing required credentials configuration")]
    MissingCredentials,

    #[error("Missing required username in credentials configuration")]
    MissingUsername,

    #[error("Missing required server configuration")]
    MissingServer,

    #[error("Missing required host in server configuration")]
    MissingHost,

    #[error("Missing required execute configuration")]
    MissingExecute,

    #[error("Missing required tasks configuration")]
    MissingTasks,

    #[error("Parent config not found:\n{0}")]
    ParentConfigNotFound(String),
}

#[derive(Error, Debug)]
pub enum ScenarioError {
    #[error("Cannot create Scenario from config:\n{0}")]
    CannotCreateScenarioFromConfig(#[source] ScenarioConfigError),

    #[error("Cannot create Execute from config:\n{0}")]
    CannotCreateExecuteFromConfig(#[source] ExecuteError),
}

#[derive(Error, Debug)]
pub enum ExecuteError {
    #[error("Cannot create Steps from config:\n{0}")]
    CannotCreateStepsFromConfig(StepsError),
}

#[derive(Error, Debug)]
pub enum StepsError {
    #[error("Cannot create Step from config:\n{0}")]
    CannotCreateStepFromConfig(StepError),

    #[error("Cannot execute step:\n{0}")]
    CannotExecuteStep(#[source] StepError),
}

#[derive(Error, Debug)]
pub enum StepError {
    #[error("Cannot execute RemoteSudo command:\n{1}:\n{0}")]
    CannotExecuteRemoteSudoCommand(#[source] RemoteSudoError, String),

    #[error("Cannot execute SftpCopy command:\n{1}:\n{0}")]
    CannotExecuteSftpCopyCommand(#[source] SftpCopyError, String),

    #[error("Cannot create OnFailSteps from config:\n{0}")]
    CannotCreateOnFailStepsFromConfig(#[source] OnFailError),

    #[error("Cannot create Task from config:\n{0}")]
    CannotCreateTaskFromConfig(String),

    #[error("Cannot execute on-fail steps:\n{0}")]
    CannotExecuteOnFailSteps(#[source] OnFailError),
}

#[derive(Error, Debug)]
pub enum OnFailError {
    #[error("OnFail step must be a valid task id:\n{0}")]
    InvalidOnFailStep(String),

    #[error("Cannot execute on-fail steps for RemoteSudo task:\n{0}")]
    CannotOnFailRemoteSudo(#[source] RemoteSudoError),

    #[error("Cannot execute on-fail steps for SftpCopy task:\n{0}")]
    CannotOnFailSftpCopy(#[source] SftpCopyError),
}

#[derive(Error, Debug)]
pub enum RemoteSudoError {
    #[error("Cannot establish a session channel:\n{0}")]
    CannotEstablishSessionChannel(#[source] ssh2::Error),

    #[error("Cannot execute remote command:\n{0}")]
    CannotExecuteRemoteCommand(#[source] ssh2::Error),

    #[error("Cannot read channel output:\n{0}")]
    CannotReadChannelOutput(#[source] ssh2::Error),

    #[error("Cannot obtain exit status of remote command:\n{0}")]
    CannotObtainRemoteCommandExitStatus(#[source] ssh2::Error),

    #[error("Remote command failed with status code:\n{0}")]
    RemoteCommandFailedWithStatusCode(i32),

    #[error("Cannot resolve placeholders in command:\n{0}")]
    CannotResolveCommandPlaceholders(#[source] PlaceholderResolutionError),

    #[error("Cannot get a lock on channel")]
    CannotGetALockOnChannel,
}

#[derive(Error, Debug)]
pub enum SftpCopyError {
    #[error("Cannot open a channel and initialize the SFTP subsystem:\n{0}")]
    CannotOpenChannelAndInitializeSftp(#[source] ssh2::Error),

    #[error("Cannot open source file:\n{0}")]
    CannotOpenSourceFile(#[source] std::io::Error),

    #[error("Cannot create a destination file:\n{0}")]
    CannotCreateDestinationFile(#[source] ssh2::Error),

    #[error("Cannot read from source file:\n{0}")]
    CannotReadSourceFile(#[source] std::io::Error),

    #[error("Cannot write to destination file:\n{0}")]
    CannotWriteDestinationFile(#[source] ssh2::Error),

    #[error("Cannot resolve placeholders in source file:\n{0}")]
    CannotResolveSourcePathPlaceholders(#[source] PlaceholderResolutionError),

    #[error("Cannot resolve placeholders in destination file:\n{0}")]
    CannotResolveDestinationPathPlaceholders(#[source] PlaceholderResolutionError),

    #[error("Cannot get a lock on SFTP channel")]
    CannotGetALockOnSftpChannel,
}

#[derive(Error, Debug)]
pub enum PlaceholderResolutionError {
    #[error("Cannot resolve placeholders in variables:\n{0:?}")]
    CannotResolveVariablesPlaceholders(Vec<String>),

    #[error("Cannot resolve placeholders in this template:\n{0}")]
    CannotResolvePlaceholders(String),
}

#[cfg(test)]
mod tests {
    use crate::scenario::errors::{
        ExecuteError, OnFailError, PlaceholderResolutionError, RemoteSudoError,
        ScenarioConfigError, ScenarioError, SftpCopyError, StepError, StepsError,
    };
    use std::io;

    fn create_io_error() -> std::io::Error {
        io::Error::new(io::ErrorKind::NotFound, "File not found")
    }

    #[test]
    fn test_scenario_config_error_display() {
        // Given
        let io_error = create_io_error();
        let config_error = ScenarioConfigError::CannotOpenConfig(io_error);

        // When
        let error_message = format!("{}", config_error);

        // Then
        assert!(error_message.contains("Cannot open config file"));
        assert!(error_message.contains("File not found"));
    }

    #[test]
    fn test_scenario_config_error_debug() {
        // Given
        let config_error = ScenarioConfigError::MissingCredentials;

        // When
        let debug_message = format!("{:?}", config_error);

        // Then
        assert!(debug_message.contains("MissingCredentials"));
    }

    #[test]
    fn test_scenario_error_display() {
        // Given
        let config_error = ScenarioConfigError::MissingServer;
        let scenario_error = ScenarioError::CannotCreateScenarioFromConfig(config_error);

        // When
        let error_message = format!("{}", scenario_error);

        // Then
        assert!(error_message.contains("Cannot create Scenario from config"));
        assert!(error_message.contains("Missing required server configuration"));
    }

    #[test]
    fn test_execute_error_display() {
        // Given
        let step_error = StepError::CannotCreateTaskFromConfig("task_id".to_string());
        let steps_error = StepsError::CannotCreateStepFromConfig(step_error);
        let execute_error = ExecuteError::CannotCreateStepsFromConfig(steps_error);

        // When
        let error_message = format!("{}", execute_error);

        // Then
        assert!(error_message.contains("Cannot create Steps from config"));
        assert!(error_message.contains("Cannot create Step from config"));
        assert!(error_message.contains("Cannot create Task from config"));
        assert!(error_message.contains("task_id"));
    }

    #[test]
    fn test_steps_error_display() {
        // Given
        let placeholder_error =
            PlaceholderResolutionError::CannotResolvePlaceholders("cmd".to_string());
        let remote_sudo_error =
            RemoteSudoError::CannotResolveCommandPlaceholders(placeholder_error);
        let step_error =
            StepError::CannotExecuteRemoteSudoCommand(remote_sudo_error, "Install App".to_string());
        let steps_error = StepsError::CannotCreateStepFromConfig(step_error);

        // When
        let error_message = format!("{}", steps_error);

        // Then
        assert!(error_message.contains("Cannot execute RemoteSudo command"));
        assert!(error_message.contains("Install App"));
        assert!(error_message.contains("Cannot resolve placeholders in command"));
    }

    #[test]
    fn test_step_error_display() {
        // Given
        let step_error = StepError::CannotCreateTaskFromConfig("invalid_id".to_string());

        // When
        let error_message = format!("{}", step_error);

        // Then
        assert!(error_message.contains("Cannot create Task from config"));
        assert!(error_message.contains("invalid_id"));
    }

    #[test]
    fn test_on_fail_error_display() {
        // Given
        let on_fail_error = OnFailError::InvalidOnFailStep("bad_task".to_string());

        // When
        let error_message = format!("{}", on_fail_error);

        // Then
        assert!(error_message.contains("OnFail step must be a valid task id"));
        assert!(error_message.contains("bad_task"));
    }

    #[test]
    fn test_remote_sudo_error_display() {
        // Given
        let remote_sudo_error = RemoteSudoError::RemoteCommandFailedWithStatusCode(127);

        // When
        let error_message = format!("{}", remote_sudo_error);

        // Then
        assert!(error_message.contains("Remote command failed with status code"));
        assert!(error_message.contains("127"));
    }

    #[test]
    fn test_sftp_copy_error_display() {
        // Given
        let io_error = create_io_error();
        let sftp_error = SftpCopyError::CannotOpenSourceFile(io_error);

        // When
        let error_message = format!("{}", sftp_error);

        // Then
        assert!(error_message.contains("Cannot open source file"));
        assert!(error_message.contains("File not found"));
    }

    #[test]
    fn test_placeholder_resolution_error_display() {
        // Given
        let unresolved_vars = vec!["var1".to_string(), "var2".to_string()];
        let error = PlaceholderResolutionError::CannotResolveVariablesPlaceholders(unresolved_vars);

        // When
        let error_message = format!("{}", error);

        // Then
        assert!(error_message.contains("Cannot resolve placeholders in variables"));
        assert!(error_message.contains("var1"));
        assert!(error_message.contains("var2"));
    }
}
