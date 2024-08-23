use crate::scenario::rollback::RollbackSteps;
use crate::scenario::tasks::Tasks;
use crate::{
    config::StepConfig,
    scenario::{
        errors::StepError
        ,
        lifecycle::StepsLifecycle,
        task::Task
        ,
    },
};
use ssh2::Session;

#[derive(Debug)]
pub struct Step {
    pub(crate) task: Task,
    pub(crate) rollback_steps: RollbackSteps,
}

impl TryFrom<(&Tasks, &StepConfig)> for Step {
    type Error = StepError;
    fn try_from((tasks, step_config): (&Tasks, &StepConfig)) -> Result<Self, Self::Error> {
        Ok(Step {
            task: tasks.get(&step_config.task).cloned()
                .ok_or_else(|| StepError::CannotCreateTaskFromConfig(
                    step_config.task.to_string()
                ))?,
            rollback_steps: match step_config.rollback_steps.as_ref() {
                Some(config) =>
                    RollbackSteps::try_from((tasks, config))
                        .map_err(StepError::CannotCreateRollbackStepsFromConfig)?,
                None => RollbackSteps::default()
            },
        })
    }
}

impl Step {
    pub fn rollback_steps(&self) -> &RollbackSteps {
        &self.rollback_steps
    }

    pub(crate) fn rollback(
        &self,
        session: &Session,
        lifecycle: &mut StepsLifecycle,
    ) -> Result<(), StepError> {
        self.rollback_steps.execute(session, &mut lifecycle.rollback)
            .map_err(StepError::CannotExecuteRollbackSteps)
    }
}
