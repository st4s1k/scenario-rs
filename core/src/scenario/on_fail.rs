use crate::config::OnFailStepsConfig;
use crate::scenario::tasks::Tasks;
use crate::scenario::variables::Variables;
use crate::scenario::{
    errors::OnFailError,
    lifecycle::OnFailLifecycle,
    task::Task,
};
use ssh2::Session;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct OnFailSteps(Vec<Task>);

impl Deref for OnFailSteps {
    type Target = Vec<Task>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OnFailSteps {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<(&Tasks, &OnFailStepsConfig)> for OnFailSteps {
    type Error = OnFailError;

    fn try_from((tasks, config): (&Tasks, &OnFailStepsConfig)) -> Result<Self, Self::Error> {
        let mut on_fail_tasks: Vec<Task> = Vec::new();
        for config_step in config.deref() {
            let task: Task = tasks.get(config_step).cloned()
                .ok_or_else(|| OnFailError::InvalidOnFailStep(config_step.clone()))?;
            on_fail_tasks.push(task);
        }
        Ok(OnFailSteps(on_fail_tasks))
    }
}

impl Default for OnFailSteps {
    fn default() -> Self {
        OnFailSteps(Vec::new())
    }
}

impl OnFailSteps {
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        lifecycle: &mut OnFailLifecycle,
    ) -> Result<(), OnFailError> {
        (lifecycle.before)(&self);

        for (index, on_fail_task) in self.iter().enumerate() {
            (lifecycle.step.before)(index, on_fail_task, self.len());
            match on_fail_task {
                Task::RemoteSudo { remote_sudo, .. } =>
                    remote_sudo.execute(&session, variables, &mut lifecycle.step.remote_sudo)
                        .map_err(OnFailError::CannotOnFailRemoteSudo)?,
                Task::SftpCopy { sftp_copy, .. } =>
                    sftp_copy.execute(&session, variables, &mut lifecycle.step.sftp_copy)
                        .map_err(OnFailError::CannotOnFailSftpCopy)?
            }
        }
        Ok(())
    }
}
