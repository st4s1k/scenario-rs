use crate::{
    config::steps::StepsConfig,
    scenario::{errors::StepsError, step::Step, task::Task, tasks::Tasks, variables::Variables},
    session::Session,
};
use std::ops::{Deref, DerefMut};
use tracing::{debug, instrument};

#[derive(Clone, Debug)]
pub struct Steps(Vec<Step>);

impl Deref for Steps {
    type Target = Vec<Step>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Steps {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<(&Tasks, &StepsConfig)> for Steps {
    type Error = StepsError;
    fn try_from((tasks, config): (&Tasks, &StepsConfig)) -> Result<Self, Self::Error> {
        let mut steps = Vec::new();
        for step_config in config.deref() {
            steps.push(
                Step::try_from((tasks, step_config))
                    .map_err(StepsError::CannotCreateStepFromConfig)?,
            );
        }
        Ok(Steps(steps))
    }
}

impl Default for Steps {
    fn default() -> Self {
        Steps(Vec::new())
    }
}

impl Steps {
    #[instrument(skip_all, name = "steps")]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
    ) -> Result<(), StepsError> {
        if self.is_empty() {
            return Ok(());
        }

        debug!(event = "steps_started");

        for (index, step) in self.iter().enumerate() {
            let total_steps = self.len();
            let description = step.task.description().to_string();

            debug!(
                event = "step_started",
                index = index,
                total_steps = total_steps,
                description = description
            );

            let error_message = step.task.error_message().to_string();

            let task_result = match &step.task {
                Task::RemoteSudo { remote_sudo, .. } => remote_sudo
                    .execute(session, variables)
                    .map_err(|error| {
                        StepsError::CannotExecuteRemoteSudoCommand(error, error_message.clone())
                    })
                    .map_err(|error| {
                        debug!(event = "error", error = %error);
                        error
                    }),
                Task::SftpCopy { sftp_copy, .. } => sftp_copy
                    .execute(session, variables)
                    .map_err(|error| {
                        StepsError::CannotExecuteSftpCopyCommand(error, error_message.clone())
                    })
                    .map_err(|error| {
                        debug!(event = "error", error = %error);
                        error
                    }),
            };

            if let Err(error) = task_result {
                step.execute_on_fail_steps(session, &variables)
                    .map_err(StepsError::CannotExecuteOnFailSteps)
                    .map_err(|error| {
                        debug!(event = "error", error = %error);
                        error
                    })?;
                return Err(error);
            }

            debug!(event = "step_completed");
        }

        debug!(event = "steps_completed");
        Ok(())
    }
}
