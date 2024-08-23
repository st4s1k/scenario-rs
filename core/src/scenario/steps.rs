use crate::{
    config::StepsConfig,
    scenario::{
        errors::StepsError,
        lifecycle::StepsLifecycle,
        step::Step,
        task::Task,
        tasks::Tasks,
    },
};
use ssh2::Session;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
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
            steps.push(Step::try_from((tasks, step_config))
                .map_err(StepsError::CannotCreateStepFromConfig)?);
        }
        Ok(Steps(steps))
    }
}

impl Steps {
    pub(crate) fn execute(
        &self,
        session: &Session,
        mut lifecycle: &mut StepsLifecycle,
    ) -> Result<(), StepsError> {
        for (index, step) in self.iter().enumerate() {
            let task = &step.task;
            (lifecycle.before)(index, task, self.len());
            let error_message = task.error_message().to_string();

            let task_result = match task {
                Task::RemoteSudo { remote_sudo, .. } =>
                    remote_sudo.execute(session, &mut lifecycle.remote_sudo)
                        .map_err(|error| StepsError::CannotExecuteRemoteSudoCommand(error, error_message)),
                Task::SftpCopy { sftp_copy, .. } =>
                    sftp_copy.execute(session, &mut lifecycle.sftp_copy)
                        .map_err(|error| StepsError::CannotExecuteSftpCopyCommand(error, error_message))
            };

            if let Err(error) = task_result {
                step.rollback(&session, &mut lifecycle)
                    .map_err(StepsError::CannotRollbackStep)?;
                return Err(error);
            };
        }

        Ok(())
    }
}
