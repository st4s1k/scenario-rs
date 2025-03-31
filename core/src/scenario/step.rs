use std::sync::mpsc::Sender;

use crate::{
    config::step::StepConfig,
    scenario::{
        errors::StepError, on_fail::OnFailSteps, task::Task, tasks::Tasks, variables::Variables,
    },
    session::Session,
};

use super::events::ScenarioEvent;

#[derive(Clone, Debug)]
pub struct Step {
    pub(crate) task: Task,
    pub(crate) on_fail_steps: OnFailSteps,
}

impl TryFrom<(&Tasks, &StepConfig)> for Step {
    type Error = StepError;
    fn try_from((tasks, step_config): (&Tasks, &StepConfig)) -> Result<Self, Self::Error> {
        let on_fail_steps = match step_config.on_fail.as_ref() {
            Some(config) => OnFailSteps::try_from((tasks, config))
                .map_err(StepError::CannotCreateOnFailStepsFromConfig)?,
            None => OnFailSteps::default(),
        };
        
        Ok(Step {
            task: tasks.get(&step_config.task).cloned().ok_or_else(|| {
                StepError::CannotCreateTaskFromConfig(step_config.task.to_string())
            })?,
            on_fail_steps,
        })
    }
}

impl Step {
    pub fn task(&self) -> &Task {
        &self.task
    }

    pub fn on_fail_steps(&self) -> &OnFailSteps {
        &self.on_fail_steps
    }

    pub(crate) fn on_fail_with_events(
        &self,
        session: &Session,
        variables: &Variables,
        tx: &Sender<ScenarioEvent>,
    ) -> Result<(), StepError> {
        self.on_fail_steps
            .execute(session, variables, tx)
            .map_err(StepError::CannotExecuteOnFailSteps)
    }
}
