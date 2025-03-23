use thiserror::Error;
use std::sync::mpsc::SendError;

use super::events::Event;

#[derive(Error, Debug)]
pub enum ScenarioConfigError {
    #[error("Cannot open config file: {0}")]
    CannotOpenConfig(#[source] std::io::Error),
    #[error("Cannot read config file: {0}")]
    CannotReadConfig(#[source] toml::de::Error),
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("Missing required credentials configuration")]
    MissingCredentials,
    #[error("Missing required server configuration")]
    MissingServer,
    #[error("Missing required execute configuration")]
    MissingExecute,
    #[error("Missing required tasks configuration")]
    MissingTasks,
    #[error("Parent config not found: {0}")]
    ParentConfigNotFound(String),
}

#[derive(Error, Debug)]
pub enum ScenarioError {
    #[error("Cannot create Scenario from config: {0}")]
    CannotCreateScenarioFromConfig(#[source] ScenarioConfigError),
    #[error("Cannot create Execute from config: {0}")]
    CannotCreateExecuteFromConfig(#[source] ExecuteError),
    #[error("Cannot connect to remote server: {0}")]
    CannotConnectToRemoteServer(#[source] std::io::Error),
    #[error("Cannot create a new session: {0}")]
    CannotCreateANewSession(#[source] ssh2::Error),
    #[error("Cannot initiate the SSH handshake: {0}")]
    CannotInitiateTheSshHandshake(#[source] ssh2::Error),
    #[error("Cannot authenticate with password: {0}")]
    CannotAuthenticateWithPassword(#[source] ssh2::Error),
    #[error("Cannot authenticate with ssh-agent: {0}")]
    CannotAuthenticateWithAgent(#[source] ssh2::Error),
    #[error("Cannot execute steps: {0}")]
    CannotExecuteSteps(#[source] StepsError),
    #[error("Cannot send scenario started event: {0}")]
    CannotSendScenarioStartedEvent(#[from] SendError<Event>),
    #[error("Cannot send scenario completed event: {0}")]
    CannotSendScenarioCompletedEvent(String),
}

#[derive(Error, Debug)]
pub enum ExecuteError {
    #[error("Cannot create Steps from config: {0}")]
    CannotCreateStepsFromConfig(StepsError),
}

#[derive(Error, Debug)]
pub enum StepsError {
    #[error("Cannot create Step from config: {0}")]
    CannotCreateStepFromConfig(StepError),
    #[error("Cannot execute RemoteSudo command: {1}: {0}")]
    CannotExecuteRemoteSudoCommand(#[source] RemoteSudoError, String),
    #[error("Cannot execute SftpCopy command: {1}: {0}")]
    CannotExecuteSftpCopyCommand(#[source] SftpCopyError, String),
    #[error("Cannot execute on-fail steps: {0}")]
    CannotExecuteOnFailSteps(#[source] StepError),
}

#[derive(Error, Debug)]
pub enum StepError {
    #[error("Cannot create OnFailSteps from config: {0}")]
    CannotCreateOnFailStepsFromConfig(#[source] OnFailError),
    #[error("Cannot create Task from config: {0}")]
    CannotCreateTaskFromConfig(String),
    #[error("Cannot execute on-fail steps: {0}")]
    CannotExecuteOnFailSteps(#[source] OnFailError),
}

#[derive(Error, Debug)]
pub enum OnFailError {
    #[error("OnFail step must be a valid task id: {0}")]
    InvalidOnFailStep(String),
    #[error("Cannot execute on-fail steps for RemoteSudo task: {0}")]
    CannotOnFailRemoteSudo(#[source] RemoteSudoError),
    #[error("Cannot execute on-fail steps for SftpCopy task: {0}")]
    CannotOnFailSftpCopy(#[source] SftpCopyError),
}

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Cannot create RemoteSudo task from config: {0}")]
    CannotCreateRemoteSudoTaskFromConfig(#[source] RemoteSudoError),
    #[error("Cannot create SftpCopy task from config: {0}")]
    CannotCreateSftpCopyTaskFromConfig(#[source] SftpCopyError),
}

#[derive(Error, Debug)]
pub enum RemoteSudoError {
    #[error("Cannot establish a session channel: {0}")]
    CannotEstablishSessionChannel(#[source] ssh2::Error),
    #[error("Cannot execute remote command: {0}")]
    CannotExecuteRemoteCommand(#[source] ssh2::Error),
    #[error("Cannot read channel output: {0}")]
    CannotReadChannelOutput(#[source] ssh2::Error),
    #[error("Cannot obtain exit status of remote command: {0}")]
    CannotObtainRemoteCommandExitStatus(#[source] ssh2::Error),
    #[error("Remote command failed with status code: {0}")]
    RemoteCommandFailedWithStatusCode(i32),
    #[error("Cannot resolve placeholders in command: {0}")]
    CannotResolveCommandPlaceholders(#[source] PlaceholderResolutionError),
    #[error("Cannot get a lock on channel")]
    CannotGetALockOnChannel,
}

#[derive(Error, Debug)]
pub enum SftpCopyError {
    #[error("Cannot open a channel and initialize the SFTP subsystem: {0}")]
    CannotOpenChannelAndInitializeSftp(#[source] ssh2::Error),
    #[error("Cannot open source file: {0}")]
    CannotOpenSourceFile(#[source] std::io::Error),
    #[error("Cannot create a destination file: {0}")]
    CannotCreateDestinationFile(#[source] ssh2::Error),
    #[error("Cannot read from source file: {0}")]
    CannotReadSourceFile(#[source] std::io::Error),
    #[error("Cannot write to destination file: {0}")]
    CannotWriteDestinationFile(#[source] ssh2::Error),
    #[error("Cannot resolve placeholders in source file: {0}")]
    CannotResolveSourcePathPlaceholders(#[source] PlaceholderResolutionError),
    #[error("Cannot resolve placeholders in destination file: {0}")]
    CannotResolveDestinationPathPlaceholders(#[source] PlaceholderResolutionError),
    #[error("Cannot get a lock on SFTP channel")]
    CannotGetALockOnSftpChannel,
}

#[derive(Error, Debug)]
pub enum PlaceholderResolutionError {
    #[error("Cannot resolve placeholders in variables: {0:?}")]
    CannotResolveVariablesPlaceholders(Vec<String>),
    #[error("Cannot resolve placeholders in: {0}")]
    CannotResolvePlaceholders(String),
}
