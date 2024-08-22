use crate::{
    config::StepConfig,
    scenario::{
        errors::TaskError,
        lifecycle::RollbackLifecycle,
        task::Task,
    },
};
use ssh2::Session;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Step {
    pub(crate) task: String,
    pub(crate) rollback_steps: Option<Vec<String>>,
}

impl From<&StepConfig> for Step {
    fn from(step_config: &StepConfig) -> Self {
        Step {
            task: step_config.task.clone(),
            rollback_steps: step_config.rollback_steps.clone(),
        }
    }
}

impl Step {
    pub fn rollback_steps(&self) -> Option<&Vec<String>> {
        self.rollback_steps.as_ref()
    }

    pub(crate) fn rollback(
        &self,
        tasks: &HashMap<String, Task>,
        session: &Session,
        lifecycle: &mut RollbackLifecycle,
    ) -> Result<(), TaskError> {
        (lifecycle.before)(&self);
        if let Some(rollback_steps) = &self.rollback_steps {
            for (index, rollback_step) in rollback_steps.iter().enumerate() {
                // TODO: Error handling - Rollback step must be a valid task
                let rollback_task = tasks.get(rollback_step).unwrap();
                (lifecycle.step.before)(index, rollback_task, rollback_steps);
                match rollback_task {
                    Task::RemoteSudo { remote_sudo, .. } =>
                        remote_sudo.execute(&session, &mut lifecycle.step.remote_sudo)
                            .map_err(TaskError::CannotRollbackRemoteSudo)?,
                    Task::SftpCopy { sftp_copy, .. } =>
                        sftp_copy.execute(&session, &mut lifecycle.step.sftp_copy)
                            .map_err(TaskError::CannotRollbackSftpCopy)?
                }
            }
        }
        Ok(())
    }
}
