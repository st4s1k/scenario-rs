//! Step execution management for scenarios.
//!
//! This module provides functionality for executing a sequence of steps
//! in a scenario, handling failures and executing fallback steps when needed.

use crate::{
    config::steps::StepsConfig,
    scenario::{errors::StepsError, step::Step, task::Task, tasks::Tasks, variables::Variables},
    session::Session,
};
use std::ops::{Deref, DerefMut};
use tracing::{debug, instrument};

/// A sequence of steps to be executed as part of a scenario.
///
/// This struct represents an ordered collection of steps that define the execution flow
/// of a scenario. Each step executes a task, and the execution sequence continues
/// until all steps complete successfully or one step fails.
///
/// # Examples
///
/// Creating steps from a configuration:
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_core::{
///     config::{
///         step::StepConfig,
///         steps::StepsConfig,
///         task::{TaskConfig, TaskType}
///     },
///     scenario::{
///         task::Task,
///         tasks::Tasks,
///         steps::Steps
///     }
/// };
///
/// // Set up the task map
/// let mut task_map = HashMap::new();
/// 
/// // Create a setup task
/// let setup_config = TaskConfig {
///     description: "Setup environment".to_string(),
///     error_message: "Setup failed".to_string(),
///     task_type: TaskType::RemoteSudo {
///         command: "mkdir -p /app/data".to_string(),
///     },
/// };
/// task_map.insert("setup".to_string(), Task::from(&setup_config));
///
/// // Create a deploy task
/// let deploy_config = TaskConfig {
///     description: "Deploy application".to_string(),
///     error_message: "Deployment failed".to_string(),
///     task_type: TaskType::SftpCopy {
///         source_path: "./app.jar".to_string(),
///         destination_path: "/app/app.jar".to_string(),
///     },
/// };
/// task_map.insert("deploy".to_string(), Task::from(&deploy_config));
///
/// // Create all available tasks
/// let tasks = Tasks(task_map);
///
/// // Define the steps configuration
/// let steps_config = StepsConfig(vec![
///     StepConfig {
///         task: "setup".to_string(),
///         on_fail: None, // No on_fail steps
///     },
///     StepConfig {
///         task: "deploy".to_string(),
///         on_fail: None, // No on_fail steps
///     },
/// ]);
///
/// // Convert to Steps
/// let result = Steps::try_from((&tasks, &steps_config));
/// assert!(result.is_ok());
/// 
/// let steps = result.unwrap();
/// assert_eq!(steps.len(), 2);
/// ```
#[derive(Clone, Debug)]
pub struct Steps(Vec<Step>);

impl Deref for Steps {
    type Target = Vec<Step>;
    
    /// Dereferences to the underlying vector of steps.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Steps {
    /// Provides mutable access to the underlying vector of steps.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<(&Tasks, &StepsConfig)> for Steps {
    type Error = StepsError;
    
    /// Attempts to create a Steps instance from tasks and steps configuration.
    ///
    /// This conversion will validate that all task references in the steps configuration
    /// exist in the provided tasks collection.
    ///
    /// # Arguments
    ///
    /// * `tasks` - Collection of available tasks
    /// * `config` - Configuration of steps to create
    ///
    /// # Returns
    ///
    /// * `Ok(Steps)` - If all referenced tasks exist and all steps were successfully created
    /// * `Err(StepsError)` - If any referenced task doesn't exist or other validation errors occur
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
    /// Creates an empty collection of steps.
    fn default() -> Self {
        Steps(Vec::new())
    }
}

impl Steps {
    /// Executes all steps in sequence, handling failures with on-fail steps.
    ///
    /// This method executes each step in order until all complete successfully or one fails.
    /// If a step fails, its on-fail steps (if any) are executed, and then execution stops
    /// with an error.
    ///
    /// # Arguments
    ///
    /// * `session` - The SSH session for executing remote operations
    /// * `variables` - Variables to use for placeholder resolution
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If all steps executed successfully
    /// * `Err(StepsError)` - If any step failed to execute
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
