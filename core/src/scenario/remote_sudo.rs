use std::{io::Read, sync::mpsc::Sender};

use crate::{
    config::RemoteSudoConfig,
    scenario::{errors::RemoteSudoError, lifecycle::RemoteSudoLifecycle, variables::Variables},
};
use ssh2::{Channel, Session};

use super::events::Event;

#[derive(Debug, Clone)]
pub struct RemoteSudo {
    pub(crate) command: String,
}

impl From<&RemoteSudoConfig> for RemoteSudo {
    fn from(config: &RemoteSudoConfig) -> Self {
        RemoteSudo {
            command: config.command.clone(),
        }
    }
}

impl RemoteSudo {
    pub fn command(&self) -> &str {
        &self.command
    }

    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        lifecycle: &mut RemoteSudoLifecycle,
    ) -> Result<(), RemoteSudoError> {
        (lifecycle.before)(&self);

        let mut channel: Channel = session
            .channel_session()
            .map_err(RemoteSudoError::CannotEstablishSessionChannel)?;
        let command = variables
            .resolve_placeholders(&self.command)
            .map_err(RemoteSudoError::CannotResolveCommandPlaceholders)?;
        channel
            .exec(&format!("{command}"))
            .map_err(RemoteSudoError::CannotExecuteRemoteCommand)?;

        (lifecycle.channel_established)(&mut channel);

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

    pub(crate) fn execute_with_events(
        &self,
        session: &Session,
        variables: &Variables,
        tx: &Sender<Event>,
    ) -> Result<(), RemoteSudoError> {
        tx.send(Event::RemoteSudoBefore(self.command.clone()))
            .expect("Failed to send RemoteSudoBefore event");

        let mut channel: Channel = session
            .channel_session()
            .map_err(RemoteSudoError::CannotEstablishSessionChannel)?;

        let command = variables
            .resolve_placeholders(&self.command)
            .map_err(RemoteSudoError::CannotResolveCommandPlaceholders)?;

        channel
            .exec(&command)
            .map_err(RemoteSudoError::CannotExecuteRemoteCommand)?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(RemoteSudoError::CannotReadChannelOutput)?;

        let truncated_output: String = output.chars().take(1000).collect();
        tx.send(Event::RemoteSudoChannelOutput(truncated_output))
            .expect("Failed to send RemoteSudoChannelOutput event");

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
