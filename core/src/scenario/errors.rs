//! Error types for the scenario system.
//!
//! This module defines all the error types that can occur during scenario
//! configuration loading, validation, and execution.

use thiserror::Error;

/// Errors that can occur when loading and processing scenario configurations.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::scenario::errors::ScenarioConfigError;
/// use std::io;
///
/// // Create an error for a file that couldn't be opened
/// let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
/// let config_error = ScenarioConfigError::CannotOpenConfig(io_error);
///
/// // Error message includes the source error
/// assert!(format!("{}", config_error).contains("Cannot open config file"));
/// ```
#[derive(Error, Debug)]
pub enum ScenarioConfigError {
    /// Failed to open the configuration file.
    #[error("Cannot open config file:\n{0}")]
    CannotOpenConfig(#[source] std::io::Error),

    /// Failed to parse the TOML in the configuration file.
    #[error("Cannot read config file:\n{0}")]
    CannotReadConfig(#[source] toml::de::Error),

    /// A circular dependency was detected in the configuration inheritance.
    #[error("Circular dependency detected:\n{0}")]
    CircularDependency(String),

    /// The required credentials section is missing from the configuration.
    #[error("Missing required credentials configuration")]
    MissingCredentials,

    /// The required username field is missing from credentials configuration.
    #[error("Missing required username in credentials configuration")]
    MissingUsername,

    /// The required server section is missing from the configuration.
    #[error("Missing required server configuration")]
    MissingServer,

    /// The required host field is missing from server configuration.
    #[error("Missing required host in server configuration")]
    MissingHost,

    /// The required execute section is missing from the configuration.
    #[error("Missing required execute configuration")]
    MissingExecute,

    /// The required tasks section is missing from the configuration.
    #[error("Missing required tasks configuration")]
    MissingTasks,

    /// A referenced parent configuration file was not found.
    #[error("Parent config not found:\n{0}")]
    ParentConfigNotFound(String),
}

/// Errors that can occur when creating or executing a scenario.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::scenario::errors::{ScenarioError, ScenarioConfigError};
///
/// // Create a scenario error that wraps a config error
/// let config_error = ScenarioConfigError::MissingCredentials;
/// let scenario_error = ScenarioError::CannotCreateScenarioFromConfig(config_error);
///
/// assert!(format!("{}", scenario_error).contains("Cannot create Scenario from config"));
/// assert!(format!("{}", scenario_error).contains("Missing required credentials"));
/// ```
#[derive(Error, Debug)]
pub enum ScenarioError {
    /// Failed to create a scenario from its configuration.
    #[error("Cannot create Scenario from config:\n{0}")]
    CannotCreateScenarioFromConfig(#[source] ScenarioConfigError),

    /// Failed to create the execution component from its configuration.
    #[error("Cannot create Execute from config:\n{0}")]
    CannotCreateExecuteFromConfig(#[source] ExecuteError),
}

/// Errors that can occur in the execution component.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::scenario::errors::{ExecuteError, StepsError, StepError};
///
/// // Create a step error
/// let step_error = StepError::CannotCreateTaskFromConfig("missing_task".to_string());
///
/// // Wrap in a steps error
/// let steps_error = StepsError::CannotCreateStepFromConfig(step_error);
///
/// // Wrap in an execute error
/// let execute_error = ExecuteError::CannotCreateStepsFromConfig(steps_error);
///
/// assert!(format!("{}", execute_error).contains("Cannot create Steps from config"));
/// ```
#[derive(Error, Debug)]
pub enum ExecuteError {
    /// Failed to create steps from their configuration.
    #[error("Cannot create Steps from config:\n{0}")]
    CannotCreateStepsFromConfig(StepsError),
}

/// Errors that can occur when executing steps.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::scenario::errors::{StepsError, StepError};
///
/// // Create a step error
/// let step_error = StepError::CannotCreateTaskFromConfig("invalid_task_id".to_string());
///
/// // Wrap in a steps error
/// let steps_error = StepsError::CannotCreateStepFromConfig(step_error);
///
/// assert!(format!("{}", steps_error).contains("Cannot create Step from config"));
/// ```
#[derive(Error, Debug)]
pub enum StepsError {
    /// Failed to create a step from its configuration.
    #[error("Cannot create Step from config:\n{0}")]
    CannotCreateStepFromConfig(StepError),

    /// Failed to execute a RemoteSudo command.
    #[error("Cannot execute RemoteSudo command:\n{1}:\n{0}")]
    CannotExecuteRemoteSudoCommand(#[source] RemoteSudoError, String),

    /// Failed to execute an SftpCopy command.
    #[error("Cannot execute SftpCopy command:\n{1}:\n{0}")]
    CannotExecuteSftpCopyCommand(#[source] SftpCopyError, String),

    /// Failed to execute on-fail steps after a step failure.
    #[error("Cannot execute on-fail steps:\n{0}")]
    CannotExecuteOnFailSteps(#[source] StepError),
}

/// Errors that can occur when creating or executing a single step.
#[derive(Error, Debug)]
pub enum StepError {
    /// Failed to create on-fail steps from their configuration.
    #[error("Cannot create OnFailSteps from config:\n{0}")]
    CannotCreateOnFailStepsFromConfig(#[source] OnFailError),

    /// Failed to create a task from its configuration.
    #[error("Cannot create Task from config:\n{0}")]
    CannotCreateTaskFromConfig(String),

    /// Failed to execute on-fail steps after a step failure.
    #[error("Cannot execute on-fail steps:\n{0}")]
    CannotExecuteOnFailSteps(#[source] OnFailError),
}

/// Errors that can occur when executing on-fail steps.
#[derive(Error, Debug)]
pub enum OnFailError {
    /// An on-fail step references an invalid task ID.
    #[error("OnFail step must be a valid task id:\n{0}")]
    InvalidOnFailStep(String),

    /// Failed to execute on-fail steps for a RemoteSudo task.
    #[error("Cannot execute on-fail steps for RemoteSudo task:\n{0}")]
    CannotOnFailRemoteSudo(#[source] RemoteSudoError),

    /// Failed to execute on-fail steps for an SftpCopy task.
    #[error("Cannot execute on-fail steps for SftpCopy task:\n{0}")]
    CannotOnFailSftpCopy(#[source] SftpCopyError),
}

/// Errors that can occur when executing a remote command with sudo privileges.
#[derive(Error, Debug)]
pub enum RemoteSudoError {
    /// Failed to establish an SSH channel session.
    #[error("Cannot establish a session channel:\n{0}")]
    CannotEstablishSessionChannel(#[source] ssh2::Error),

    /// Failed to execute the remote command.
    #[error("Cannot execute remote command:\n{0}")]
    CannotExecuteRemoteCommand(#[source] ssh2::Error),

    /// Failed to read the output of the remote command.
    #[error("Cannot read channel output:\n{0}")]
    CannotReadChannelOutput(#[source] ssh2::Error),

    /// Failed to get the exit status of the remote command.
    #[error("Cannot obtain exit status of remote command:\n{0}")]
    CannotObtainRemoteCommandExitStatus(#[source] ssh2::Error),

    /// The remote command executed but returned a non-zero exit code.
    #[error("Remote command failed with status code:\n{0}")]
    RemoteCommandFailedWithStatusCode(i32),

    /// Failed to resolve variable placeholders in the command string.
    #[error("Cannot resolve placeholders in command:\n{0}")]
    CannotResolveCommandPlaceholders(#[source] PlaceholderResolutionError),

    /// Failed to obtain a lock on the SSH channel.
    #[error("Cannot get a lock on channel")]
    CannotGetALockOnChannel,
}

/// Errors that can occur when copying files via SFTP.
#[derive(Error, Debug)]
pub enum SftpCopyError {
    /// Failed to open an SFTP session.
    #[error("Cannot open a channel and initialize the SFTP subsystem:\n{0}")]
    CannotOpenChannelAndInitializeSftp(#[source] ssh2::Error),

    /// Failed to open the source file for reading.
    #[error("Cannot open source file:\n{0}")]
    CannotOpenSourceFile(#[source] std::io::Error),

    /// Failed to create the destination file on the remote server.
    #[error("Cannot create a destination file:\n{0}")]
    CannotCreateDestinationFile(#[source] ssh2::Error),

    /// Failed to read from the source file.
    #[error("Cannot read from source file:\n{0}")]
    CannotReadSourceFile(#[source] std::io::Error),

    /// Failed to write to the destination file.
    #[error("Cannot write to destination file:\n{0}")]
    CannotWriteDestinationFile(#[source] ssh2::Error),

    /// Failed to resolve variable placeholders in the source path.
    #[error("Cannot resolve placeholders in source file:\n{0}")]
    CannotResolveSourcePathPlaceholders(#[source] PlaceholderResolutionError),

    /// Failed to resolve variable placeholders in the destination path.
    #[error("Cannot resolve placeholders in destination file:\n{0}")]
    CannotResolveDestinationPathPlaceholders(#[source] PlaceholderResolutionError),

    /// Failed to obtain a lock on the SFTP channel.
    #[error("Cannot get a lock on SFTP channel")]
    CannotGetALockOnSftpChannel,
}

/// Errors that can occur when resolving variable placeholders.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::scenario::errors::PlaceholderResolutionError;
///
/// // Create an error for unresolvable placeholders
/// let template = "Hello, {missing_var}!";
/// let error = PlaceholderResolutionError::CannotResolvePlaceholders(template.to_string());
///
/// assert!(format!("{}", error).contains("Cannot resolve placeholders in this template"));
/// assert!(format!("{}", error).contains("Hello, {missing_var}!"));
/// ```
#[derive(Error, Debug)]
pub enum PlaceholderResolutionError {
    /// Failed to resolve placeholders in variable values, creating circular references.
    #[error("Cannot resolve placeholders in variables:\n{0:?}")]
    CannotResolveVariablesPlaceholders(Vec<String>),

    /// Failed to resolve placeholders in a string template.
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

    // Test helpers
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
        let steps_error = StepsError::CannotExecuteRemoteSudoCommand(
            remote_sudo_error,
            "Install App".to_string(),
        );

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
