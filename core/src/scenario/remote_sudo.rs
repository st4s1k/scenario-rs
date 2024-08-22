use crate::{
    config::RemoteSudoConfig,
    scenario::{
        credentials::Credentials,
        errors::RemoteSudoError,
        lifecycle::RemoteSudoLifecycle,
        variables::Variables,
    },
};
use ssh2::{Channel, Session};

pub struct RemoteSudo {
    pub(crate) command: String,
}

impl TryFrom<(&RemoteSudoConfig, &Variables)> for RemoteSudo {
    type Error = RemoteSudoError;

    fn try_from((config, variables): (&RemoteSudoConfig, &Variables)) -> Result<Self, Self::Error> {
        let command = variables.resolve_placeholders(&config.command)
            .map_err(RemoteSudoError::CannotResolveCommandPlaceholders)?;

        Ok(RemoteSudo { command })
    }
}

impl RemoteSudo {
    pub fn command(&self) -> &str {
        &self.command
    }

    pub(crate) fn execute(
        &self,
        credentials: &Credentials,
        session: &Session,
        lifecycle: &mut RemoteSudoLifecycle,
    ) -> Result<(), RemoteSudoError> {
        (lifecycle.before)(&self);

        let mut channel: Channel = session.channel_session()
            .map_err(RemoteSudoError::CannotEstablishSessionChannel)?;
        let password = &credentials.password;
        let command = &self.command;
        channel.exec(&format!("echo {password} | sudo -S {command}"))
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
