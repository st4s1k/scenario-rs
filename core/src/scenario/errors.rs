//! Error types for the scenario system.
//!
//! This module defines all the error types that can occur during scenario
//! configuration loading, validation, and execution.

use thiserror::Error;

/// Errors that can occur when loading and processing scenario configurations.
#[derive(Error, Debug)]
pub enum ScenarioConfigError {
    /// Failed to open the configuration file.
    #[error("Cannot open config file: {0}")]
    CannotOpenConfig(#[source] std::io::Error),
    
    /// Failed to parse the TOML in the configuration file.
    #[error("Cannot read config file: {0}")]
    CannotReadConfig(#[source] toml::de::Error),
    
    /// A circular dependency was detected in the configuration inheritance.
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    
    /// The required credentials section is missing from the configuration.
    #[error("Missing required credentials configuration")]
    MissingCredentials,
    
    /// The required server section is missing from the configuration.
    #[error("Missing required server configuration")]
    MissingServer,
    
    /// The required execute section is missing from the configuration.
    #[error("Missing required execute configuration")]
    MissingExecute,
    
    /// The required tasks section is missing from the configuration.
    #[error("Missing required tasks configuration")]
    MissingTasks,
    
    /// A referenced parent configuration file was not found.
    #[error("Parent config not found: {0}")]
    ParentConfigNotFound(String),
}

/// Errors that can occur when creating or executing a scenario.
#[derive(Error, Debug)]
pub enum ScenarioError {
    /// Failed to create a scenario from its configuration.
    #[error("Cannot create Scenario from config: {0}")]
    CannotCreateScenarioFromConfig(#[source] ScenarioConfigError),
    
    /// Failed to create the execution component from its configuration.
    #[error("Cannot create Execute from config: {0}")]
    CannotCreateExecuteFromConfig(#[source] ExecuteError),
}

/// Errors that can occur in the execution component.
#[derive(Error, Debug)]
pub enum ExecuteError {
    /// Failed to create steps from their configuration.
    #[error("Cannot create Steps from config: {0}")]
    CannotCreateStepsFromConfig(StepsError),
}

/// Errors that can occur when executing steps.
#[derive(Error, Debug)]
pub enum StepsError {
    /// Failed to create a step from its configuration.
    #[error("Cannot create Step from config: {0}")]
    CannotCreateStepFromConfig(StepError),
    
    /// Failed to execute a RemoteSudo command.
    #[error("Cannot execute RemoteSudo command: {1}: {0}")]
    CannotExecuteRemoteSudoCommand(#[source] RemoteSudoError, String),
    
    /// Failed to execute an SftpCopy command.
    #[error("Cannot execute SftpCopy command: {1}: {0}")]
    CannotExecuteSftpCopyCommand(#[source] SftpCopyError, String),
    
    /// Failed to execute on-fail steps after a step failure.
    #[error("Cannot execute on-fail steps: {0}")]
    CannotExecuteOnFailSteps(#[source] StepError),
}

/// Errors that can occur when creating or executing a single step.
#[derive(Error, Debug)]
pub enum StepError {
    /// Failed to create on-fail steps from their configuration.
    #[error("Cannot create OnFailSteps from config: {0}")]
    CannotCreateOnFailStepsFromConfig(#[source] OnFailError),
    
    /// Failed to create a task from its configuration.
    #[error("Cannot create Task from config: {0}")]
    CannotCreateTaskFromConfig(String),
    
    /// Failed to execute on-fail steps after a step failure.
    #[error("Cannot execute on-fail steps: {0}")]
    CannotExecuteOnFailSteps(#[source] OnFailError),
}

/// Errors that can occur when executing on-fail steps.
#[derive(Error, Debug)]
pub enum OnFailError {
    /// An on-fail step references an invalid task ID.
    #[error("OnFail step must be a valid task id: {0}")]
    InvalidOnFailStep(String),
    
    /// Failed to execute on-fail steps for a RemoteSudo task.
    #[error("Cannot execute on-fail steps for RemoteSudo task: {0}")]
    CannotOnFailRemoteSudo(#[source] RemoteSudoError),
    
    /// Failed to execute on-fail steps for an SftpCopy task.
    #[error("Cannot execute on-fail steps for SftpCopy task: {0}")]
    CannotOnFailSftpCopy(#[source] SftpCopyError),
}

/// Errors that can occur when executing a remote command with sudo privileges.
#[derive(Error, Debug)]
pub enum RemoteSudoError {
    /// Failed to establish an SSH channel session.
    #[error("Cannot establish a session channel: {0}")]
    CannotEstablishSessionChannel(#[source] ssh2::Error),
    
    /// Failed to execute the remote command.
    #[error("Cannot execute remote command: {0}")]
    CannotExecuteRemoteCommand(#[source] ssh2::Error),
    
    /// Failed to read the output of the remote command.
    #[error("Cannot read channel output: {0}")]
    CannotReadChannelOutput(#[source] ssh2::Error),
    
    /// Failed to get the exit status of the remote command.
    #[error("Cannot obtain exit status of remote command: {0}")]
    CannotObtainRemoteCommandExitStatus(#[source] ssh2::Error),
    
    /// The remote command executed but returned a non-zero exit code.
    #[error("Remote command failed with status code: {0}")]
    RemoteCommandFailedWithStatusCode(i32),
    
    /// Failed to resolve variable placeholders in the command string.
    #[error("Cannot resolve placeholders in command: {0}")]
    CannotResolveCommandPlaceholders(#[source] PlaceholderResolutionError),
    
    /// Failed to obtain a lock on the SSH channel.
    #[error("Cannot get a lock on channel")]
    CannotGetALockOnChannel,
}

/// Errors that can occur when copying files via SFTP.
#[derive(Error, Debug)]
pub enum SftpCopyError {
    /// Failed to open an SFTP session.
    #[error("Cannot open a channel and initialize the SFTP subsystem: {0}")]
    CannotOpenChannelAndInitializeSftp(#[source] ssh2::Error),
    
    /// Failed to open the source file for reading.
    #[error("Cannot open source file: {0}")]
    CannotOpenSourceFile(#[source] std::io::Error),
    
    /// Failed to create the destination file on the remote server.
    #[error("Cannot create a destination file: {0}")]
    CannotCreateDestinationFile(#[source] ssh2::Error),
    
    /// Failed to read from the source file.
    #[error("Cannot read from source file: {0}")]
    CannotReadSourceFile(#[source] std::io::Error),
    
    /// Failed to write to the destination file.
    #[error("Cannot write to destination file: {0}")]
    CannotWriteDestinationFile(#[source] ssh2::Error),
    
    /// Failed to resolve variable placeholders in the source path.
    #[error("Cannot resolve placeholders in source file: {0}")]
    CannotResolveSourcePathPlaceholders(#[source] PlaceholderResolutionError),
    
    /// Failed to resolve variable placeholders in the destination path.
    #[error("Cannot resolve placeholders in destination file: {0}")]
    CannotResolveDestinationPathPlaceholders(#[source] PlaceholderResolutionError),
    
    /// Failed to obtain a lock on the SFTP channel.
    #[error("Cannot get a lock on SFTP channel")]
    CannotGetALockOnSftpChannel,
}

/// Errors that can occur when resolving variable placeholders.
#[derive(Error, Debug)]
pub enum PlaceholderResolutionError {
    /// Failed to resolve placeholders in variable values, creating circular references.
    #[error("Cannot resolve placeholders in variables: {0:?}")]
    CannotResolveVariablesPlaceholders(Vec<String>),
    
    /// Failed to resolve placeholders in a string template.
    #[error("Cannot resolve placeholders in this template: {0}")]
    CannotResolvePlaceholders(String),
}
