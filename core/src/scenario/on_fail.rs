use crate::config::OnFailStepsConfig;
use crate::scenario::tasks::Tasks;
use crate::scenario::variables::Variables;
use crate::scenario::{errors::OnFailError, task::Task};
use ssh2::Session;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::Sender;

use super::events::Event;

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
            let task: Task = tasks
                .get(config_step)
                .cloned()
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
        tx: &Sender<Event>,
    ) -> Result<(), OnFailError> {
        tx.send(Event::OnFailStepsStarted)
            .expect("Failed to send OnFailStepsStarted event");

        for (index, on_fail_task) in self.iter().enumerate() {
            tx.send(Event::OnFailStepStarted {
                index,
                total_steps: self.len(),
                description: on_fail_task.description().to_string(),
            })
            .expect("Failed to send OnFailStepStarted event");

            match on_fail_task {
                Task::RemoteSudo { remote_sudo, .. } => remote_sudo
                    .execute(session, variables, tx)
                    .map_err(OnFailError::CannotOnFailRemoteSudo)?,
                Task::SftpCopy { sftp_copy, .. } => sftp_copy
                    .execute(session, variables, tx)
                    .map_err(OnFailError::CannotOnFailSftpCopy)?,
            }
        }
        Ok(())
    }
}
