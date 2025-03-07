use crate::scenario::variables::Variables;
use crate::{
    config::StepsConfig,
    scenario::{
        errors::StepsError, step::Step, task::Task, tasks::Tasks,
    },
};
use ssh2::Session;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::Sender;

use super::events::Event;

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
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        tx: &Sender<Event>,
    ) -> Result<(), StepsError> {
        for (index, step) in self.iter().enumerate() {
            tx.send(Event::StepStarted {
                index,
                total_steps: self.len(),
                description: step.task.description().to_string(),
            })
            .unwrap();

            let error_message = step.task.error_message().to_string();

            let task_result = match &step.task {
                Task::RemoteSudo { remote_sudo, .. } => {
                    tx.send(Event::RemoteSudoBefore(remote_sudo.command().to_string()))
                        .unwrap();
                    remote_sudo
                        .execute(session, variables, tx)
                        .map_err(|e| {
                            StepsError::CannotExecuteRemoteSudoCommand(e, error_message.clone())
                        })
                }
                Task::SftpCopy { sftp_copy, .. } => {
                    tx.send(Event::SftpCopyBefore {
                        source: sftp_copy.source_path().to_string(),
                        destination: sftp_copy.destination_path().to_string(),
                    })
                    .unwrap();
                    sftp_copy
                        .execute(session, variables, tx)
                        .map_err(|e| {
                            StepsError::CannotExecuteSftpCopyCommand(e, error_message.clone())
                        })
                }
            };

            if let Err(err) = task_result {
                tx.send(Event::ScenarioError(format!("Step error: {}", err)))
                    .unwrap();
                step.on_fail_with_events(session, variables, tx)
                    .map_err(StepsError::CannotExecuteOnFailSteps)?;
                return Err(err);
            }
        }
        Ok(())
    }
}
