//! Step execution handling for scenarios.
//!
//! This module provides functionality for executing individual steps
//! in a scenario, including regular tasks and fallback steps for error handling.

use crate::{
    config::step::StepConfig,
    scenario::{
        errors::StepError, on_fail::OnFailSteps, task::Task, tasks::Tasks, variables::Variables,
    },
    session::Session,
};

/// A single step to be executed as part of a scenario.
///
/// A step represents a single operation in a scenario execution flow. It consists of a
/// task to be executed and optional fallback steps to run if the primary task fails.
/// This structure enables graceful error handling and cleanup actions.
#[derive(Clone, Debug)]
pub struct Step {
    /// The primary task to be executed
    pub(crate) task: Task,
    /// Steps to execute if the primary task fails
    pub(crate) on_fail_steps: OnFailSteps,
}

impl TryFrom<(&Tasks, &StepConfig)> for Step {
    type Error = StepError;
    
    /// Attempts to create a Step instance from tasks and step configuration.
    ///
    /// This conversion will validate that the referenced task exists in the
    /// provided tasks collection and that any on-fail steps are valid.
    ///
    /// # Arguments
    ///
    /// * `tasks` - Collection of available tasks
    /// * `step_config` - Configuration for this step
    ///
    /// # Returns
    ///
    /// * `Ok(Step)` - If the referenced task exists and on-fail steps are valid
    /// * `Err(StepError)` - If the referenced task doesn't exist or on-fail steps are invalid
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
    /// Returns a reference to the step's task.
    pub fn task(&self) -> &Task {
        &self.task
    }

    /// Returns a reference to the step's on-fail steps.
    pub fn on_fail_steps(&self) -> &OnFailSteps {
        &self.on_fail_steps
    }

    /// Executes the on-fail steps for this step.
    ///
    /// This method is called when the primary task fails, to perform any
    /// necessary cleanup or recovery actions.
    ///
    /// # Arguments
    ///
    /// * `session` - The SSH session for executing remote operations
    /// * `variables` - Variables to use for placeholder resolution
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If all on-fail steps executed successfully
    /// * `Err(StepError)` - If any on-fail step failed to execute
    pub(crate) fn execute_on_fail_steps(
        &self,
        session: &Session,
        variables: &Variables,
    ) -> Result<(), StepError> {
        self.on_fail_steps
            .execute(session, variables)
            .map_err(StepError::CannotExecuteOnFailSteps)
    }
}
