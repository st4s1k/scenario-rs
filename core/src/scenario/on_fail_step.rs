use crate::{
    scenario::{errors::OnFailError, task::Task, variables::Variables},
    session::Session,
};
use tracing::{debug, instrument};

#[derive(Clone, Debug)]
pub struct OnFailStep {
    /// The index of the on-fail step in the step
    pub(crate) index: usize,
    /// The primary task to be executed
    pub(crate) task: Task,
}

impl From<(usize, Task)> for OnFailStep {
    fn from((index, task): (usize, Task)) -> Self {
        Self { index, task }
    }
}

impl OnFailStep {
    /// Returns the index of the on-fail step
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the task associated with this on-fail step
    pub fn task(&self) -> &Task {
        &self.task
    }

    /// Execute the on-fail step
    #[instrument(
        name = "on_fail_step",
        skip_all,
        fields(on_fail_step.index = self.index)
    )]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
    ) -> Result<(), OnFailError> {
        debug!(
            scenario.event = "on_fail_step_started",
            task.description = self.task.description()
        );

        let result = match &self.task {
            Task::RemoteSudo { remote_sudo, .. } => remote_sudo
                .execute(session, variables)
                .map_err(OnFailError::CannotOnFailRemoteSudo)
                .map_err(|error| {
                    debug!(scenario.event = "error", scenario.error = %error);
                    error
                }),
            Task::SftpCopy { sftp_copy, .. } => sftp_copy
                .execute(session, variables)
                .map_err(OnFailError::CannotOnFailSftpCopy)
                .map_err(|error| {
                    debug!(scenario.event = "error", scenario.error = %error);
                    error
                }),
        };

        if result.is_ok() {
            debug!(scenario.event = "on_fail_step_completed");
        }

        result
    }
}
