use std::sync::mpsc::Sender;

use crate::{
    scenario::{errors::RemoteSudoError, utils::SendEvent, variables::Variables},
    session::Session,
};

use super::events::Event;

/// Represents a remote command to be executed with sudo privileges
///
/// This struct holds a command string to be executed on a remote session
/// with elevated permissions.
#[derive(Debug, Clone)]
pub struct RemoteSudo {
    pub(crate) command: String,
}

impl RemoteSudo {
    /// Returns a reference to the command string
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Executes the sudo command on the remote session
    ///
    /// # Arguments
    ///
    /// * `session` - The session to execute the command on
    /// * `variables` - Variables to resolve placeholders in the command
    /// * `tx` - Channel to send events during execution
    ///
    /// # Returns
    ///
    /// `Ok(())` if the command executed successfully with exit code 0,
    /// otherwise an appropriate `RemoteSudoError`
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        tx: &Sender<Event>,
    ) -> Result<(), RemoteSudoError> {
        let command = variables
            .resolve_placeholders(&self.command)
            .map_err(RemoteSudoError::CannotResolveCommandPlaceholders)?;

        tx.send_event(Event::RemoteSudoBefore(command.clone()));

        let mut channel = session
            .channel_session()
            .map_err(RemoteSudoError::CannotEstablishSessionChannel)?;

        channel
            .exec(&command)
            .map_err(RemoteSudoError::CannotExecuteRemoteCommand)?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(RemoteSudoError::CannotReadChannelOutput)?;

        let truncated_output: String = output.chars().take(1000).collect();

        tx.send_event(Event::RemoteSudoChannelOutput(truncated_output));

        let exit_status = channel
            .exit_status()
            .map_err(RemoteSudoError::CannotObtainRemoteCommandExitStatus)?;

        if exit_status != 0 {
            return Err(RemoteSudoError::RemoteCommandFailedWithStatusCode(
                exit_status,
            ));
        }

        Ok(())
    }
}
