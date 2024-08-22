use crate::config::ScenarioConfig;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScenarioConfigError {
    #[error("Cannot open config file: {0}")]
    CannotOpenFile(#[source] std::io::Error),
    #[error("Cannot read JSON config file: {0}")]
    CannotReadJson(#[source] serde_json::Error),
    #[error("Undefined required variables detected: {1:?}")]
    UndefinedRequiredVariablesDetected(ScenarioConfig, Vec<String>),
}

#[derive(Error, Debug)]
pub enum ScenarioError {
    #[error("Cannot create Variables from config: {0}")]
    CannotCreateVariablesFromConfig(#[source] VariablesError),
    #[error("Cannot resolve placeholders in tasks: {0}")]
    CannotResolvePlaceholdersInTasks(#[source] TasksError),
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
    #[error("Cannot execute RemoteSudo command: {1}: {0}")]
    CannotExecuteRemoteSudoCommand(#[source] RemoteSudoError, String),
    #[error("Cannot execute SftpCopy command: {1}: {0}")]
    CannotExecuteSftpCopyCommand(#[source] SftpCopyError, String),
    #[error("Cannot rollback task: {0}")]
    CannotRollbackTask(#[source] TaskError),
}

#[derive(Error, Debug)]
pub enum TasksError {
    #[error("Cannot resolve placeholders in tasks: {0:?}")]
    CannotResolvePlaceholdersInTasks(Vec<String>),
}

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Cannot resolve placeholders in RemoteSudo: {0}")]
    CannotResolveRemoteSudoPlaceholders(#[source] RemoteSudoError),
    #[error("Cannot resolve placeholders in SftpCopy: {0}")]
    CannotResolveSftpCopyPlaceholders(#[source] SftpCopyError),
    #[error("Cannot rollback RemoteSudo task: {0}")]
    CannotRollbackRemoteSudo(#[source] RemoteSudoError),
    #[error("Cannot rollback SftpCopy task: {0}")]
    CannotRollbackSftpCopy(#[source] SftpCopyError),
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
    #[error("Cannot obtain exit status of remote command: {0}")]
    CannotObtainRemoteCommandExitStatus(#[source] ssh2::Error),
    #[error("Remote command failed with status code: {0}")]
    RemoteCommandFailedWithStatusCode(i32),
    #[error("Cannot resolve placeholders in command: {0}")]
    CannotResolveCommandPlaceholders(#[source] PlaceholderResolutionError),
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
    CannotWriteDestinationFile(#[source] std::io::Error),
    #[error("Cannot resolve placeholders in source file: {0}")]
    CannotResolveSourcePathPlaceholders(#[source] PlaceholderResolutionError),
    #[error("Cannot resolve placeholders in destination file: {0}")]
    CannotResolveDestinationPathPlaceholders(#[source] PlaceholderResolutionError),
}

#[derive(Error, Debug)]
pub enum VariablesError {
    #[error("Cannot resolve placeholders in variables: {0}")]
    CannotResolveVariablesPlaceholders(#[source] PlaceholderResolutionError),
}

#[derive(Error, Debug)]
pub enum PlaceholderResolutionError {
    #[error("An unresolved value detected: {0}")]
    UnresolvedValue(String),
    #[error("Unresolved values detected: {0:?}")]
    UnresolvedValues(Vec<String>),
}
