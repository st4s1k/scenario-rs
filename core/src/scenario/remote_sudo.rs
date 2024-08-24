use crate::{
    config::RemoteSudoConfig,
    scenario::{
        errors::RemoteSudoError,
        lifecycle::RemoteSudoLifecycle,
        variables::Variables,
    },
};
use ssh2::{Channel, Session};

#[derive(Debug, Clone)]
pub struct RemoteSudo {
    pub(crate) command: String,
}

impl From<&RemoteSudoConfig> for RemoteSudo {
    fn from(config: &RemoteSudoConfig) -> Self {
        RemoteSudo { command: config.command.clone() }
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

        let mut channel: Channel = session.channel_session()
            .map_err(RemoteSudoError::CannotEstablishSessionChannel)?;
        let command = variables.resolve_placeholders(&self.command)
            .map_err(RemoteSudoError::CannotResolveCommandPlaceholders)?;
        channel.exec(&format!("{command}"))
            .map_err(RemoteSudoError::CannotExecuteRemoteCommand)?;

        (lifecycle.channel_established)(&mut channel);

        let exit_status = channel.exit_status()
            .map_err(RemoteSudoError::CannotObtainRemoteCommandExitStatus)?;

        if exit_status != 0 {
            return Err(RemoteSudoError::RemoteCommandFailedWithStatusCode(exit_status));
        }

        Ok(())
    }
}
