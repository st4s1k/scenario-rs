use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScenarioConfigError {
    #[error("Cannot open config file")]
    CannotOpenFile(#[source] std::io::Error),
    #[error("Cannot read JSON config file")]
    CannotReadJson(#[source] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum ScenarioError {
    #[error("Cannot create Variables from config")]
    CannotCreateVariablesFromConfig(#[source] VariablesError),
    #[error("Cannot create Tasks from config")]
    CannotCreateTasksFromConfig(#[source] TasksError),
    #[error("Cannot connect to remote server")]
    CannotConnectToRemoteServer(#[source] std::io::Error),
    #[error("Cannot create a new session")]
    CannotCreateANewSession(#[source] ssh2::Error),
    #[error("Cannot initiate the SSH handshake")]
    CannotInitiateTheSshHandshake(#[source] ssh2::Error),
    #[error("Cannot authenticate with password")]
    CannotAuthenticateWithPassword(#[source] ssh2::Error),
    #[error("Cannot execute RemoteSudo command: {1}")]
    CannotExecuteRemoteSudoCommand(#[source] RemoteSudoError, String),
    #[error("Cannot execute SftpCopy command: {1}")]
    CannotExecuteSftpCopyCommand(#[source] SftpCopyError, String),
    #[error("Cannot rollback task")]
    CannotRollbackTask(#[source] TaskError),
}

#[derive(Error, Debug)]
pub enum TasksError {
    #[error("Cannot create task from config")]
    CannotCreateTaskFromConfig(#[source] TaskError)
}

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Cannot resolve placeholders in RemoteSudo")]
    CannotRollbackRemoteSudo(#[source] RemoteSudoError),
    #[error("Cannot resolve placeholders in SftpCopy")]
    CannotRollbackSftpCopy(#[source] SftpCopyError),
    #[error("Cannot create RemoteSudo task from config")]
    CannotCreateRemoteSudoTaskFromConfig(#[source] RemoteSudoError),
    #[error("Cannot create SftpCopy task from config")]
    CannotCreateSftpCopyTaskFromConfig(#[source] SftpCopyError),
}

#[derive(Error, Debug)]
pub enum RemoteSudoError {
    #[error("Cannot establish a session channel")]
    CannotEstablishSessionChannel(#[source] ssh2::Error),
    #[error("Cannot execute remote command")]
    CannotExecuteRemoteCommand(#[source] ssh2::Error),
    #[error("Cannot obtain exit status of remote command")]
    CannotObtainRemoteCommandExitStatus(#[source] ssh2::Error),
    #[error("Remote command failed with status code {0}")]
    RemoteCommandFailedWithStatusCode(i32),
    #[error("Cannot resolve placeholders in command")]
    CannotResolveCommandPlaceholders(#[source] PlaceholderResolutionError),
}

#[derive(Error, Debug)]
pub enum SftpCopyError {
    #[error("Cannot open a channel and initialize the SFTP subsystem")]
    CannotOpenChannelAndInitializeSftp(#[source] ssh2::Error),
    #[error("Cannot open source file")]
    CannotOpenSourceFile(#[source] std::io::Error),
    #[error("Cannot create a destination file")]
    CannotCreateDestinationFile(#[source] ssh2::Error),
    #[error("Cannot read from source file")]
    CannotReadSourceFile(#[source] std::io::Error),
    #[error("Cannot write to destination file")]
    CannotWriteDestinationFile(#[source] std::io::Error),
    #[error("Cannot resolve placeholders in source file")]
    CannotResolveSourcePathPlaceholders(#[source] PlaceholderResolutionError),
    #[error("Cannot resolve placeholders in destination file")]
    CannotResolveDestinationPathPlaceholders(#[source] PlaceholderResolutionError),
}

#[derive(Error, Debug)]
pub enum VariablesError {
    #[error("Validation failed for required variables")]
    RequiredVariablesValidationFailed(#[source] RequiredVariablesError),
    #[error("Cannot resolve placeholders in variables")]
    CannotResolveVariablesPlaceholders(#[source] PlaceholderResolutionError),
}

#[derive(Error, Debug)]
pub enum PlaceholderResolutionError {
    #[error("An unresolved value detected")]
    UnresolvedValue(String),
    #[error("Unresolved values detected")]
    UnresolvedValues(Vec<String>),
}

#[derive(Error, Debug)]
pub enum RequiredVariablesError {
    #[error("Validation failed for required variables")]
    ValidationFailed(Vec<String>, Vec<String>),
}
