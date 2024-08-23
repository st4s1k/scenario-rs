use crate::config::RollbackStepsConfig;
use crate::scenario::tasks::Tasks;
use crate::scenario::{
    errors::RollbackError,
    lifecycle::RollbackLifecycle,
    task::Task,
};
use ssh2::Session;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct RollbackSteps(Vec<Task>);

impl Deref for RollbackSteps {
    type Target = Vec<Task>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RollbackSteps {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<(&Tasks, &RollbackStepsConfig)> for RollbackSteps {
    type Error = RollbackError;

    fn try_from((tasks, config): (&Tasks, &RollbackStepsConfig)) -> Result<Self, Self::Error> {
        let mut rollback_tasks: Vec<Task> = Vec::new();
        for config_step in config.deref() {
            let task: Task = tasks.get(config_step).cloned()
                .ok_or_else(|| RollbackError::InvalidRollbackStep(config_step.clone()))?;
            rollback_tasks.push(task);
        }
        Ok(RollbackSteps(rollback_tasks))
    }
}

impl Default for RollbackSteps {
    fn default() -> Self {
        RollbackSteps(Vec::new())
    }
}

impl RollbackSteps {
    pub(crate) fn execute(
        &self,
        session: &Session,
        lifecycle: &mut RollbackLifecycle,
    ) -> Result<(), RollbackError> {
        (lifecycle.before)(&self);

        for (index, rollback_task) in self.iter().enumerate() {
            (lifecycle.step.before)(index, rollback_task, self.len());
            match rollback_task {
                Task::RemoteSudo { remote_sudo, .. } =>
                    remote_sudo.execute(&session, &mut lifecycle.step.remote_sudo)
                        .map_err(RollbackError::CannotRollbackRemoteSudo)?,
                Task::SftpCopy { sftp_copy, .. } =>
                    sftp_copy.execute(&session, &mut lifecycle.step.sftp_copy)
                        .map_err(RollbackError::CannotRollbackSftpCopy)?
            }
        }
        Ok(())
    }
}
