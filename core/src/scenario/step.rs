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
use tracing::{debug, instrument};

/// A single step to be executed as part of a scenario.
///
/// A step represents a single operation in a scenario execution flow. It consists of a
/// task to be executed and optional fallback steps to run if the primary task fails.
/// This structure enables graceful error handling and cleanup actions.
///
/// # Examples
///
/// Creating a step with on-fail steps:
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_core::{
///     config::step::StepConfig,
///     scenario::{
///         step::Step,
///         task::Task,
///         tasks::Tasks,
///         on_fail::OnFailSteps
///     },
///     config::task::{TaskConfig, TaskType}
/// };
///
/// // Set up the task map with main and recovery tasks
/// let mut task_map = HashMap::new();
///
/// // Main deployment task
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
/// // Cleanup task for error recovery
/// let cleanup_config = TaskConfig {
///     description: "Clean up failed deployment".to_string(),
///     error_message: "Cleanup failed".to_string(),
///     task_type: TaskType::RemoteSudo {
///         command: "rm -f /app/app.jar".to_string(),
///     },
/// };
/// task_map.insert("cleanup".to_string(), Task::from(&cleanup_config));
///
/// // Create all available tasks
/// let tasks = Tasks::from(task_map);
///
/// // Define a step configuration
/// // Note: For testing, we avoid creating OnFailStepsConfig directly since its constructor is private
/// let task_name = "deploy".to_string();
/// let step_config = StepConfig {
///     task: task_name.clone(),
///     on_fail: None, // We'll handle on_fail steps differently
/// };
///
/// // Create the step
/// let mut step = Step::try_from((0, &tasks, &step_config)).unwrap();
///
/// // Create on_fail steps manually using the public API
/// let mut on_fail_steps = OnFailSteps::default();
/// if let Some(cleanup_task) = tasks.get("cleanup") {
///     on_fail_steps.push(cleanup_task.clone());
/// }
///
/// // Verify the step properties
/// assert_eq!(step.task().description(), "Deploy application");
/// ```
#[derive(Clone, Debug)]
pub struct Step {
    /// The index of the step in the scenario
    pub(crate) index: usize,
    /// The primary task to be executed
    pub(crate) task: Task,
    /// Steps to execute if the primary task fails
    pub(crate) on_fail_steps: OnFailSteps,
}

impl TryFrom<(usize, &Tasks, &StepConfig)> for Step {
    type Error = StepError;

    /// Attempts to create a Step instance from tasks and step configuration.
    ///
    /// This conversion will validate that the referenced task exists in the
    /// provided tasks collection and that any on-fail steps are valid.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the step in the scenario
    /// * `tasks` - Collection of available tasks
    /// * `step_config` - Configuration for this step
    ///
    /// # Returns
    ///
    /// * `Ok(Step)` - If the referenced task exists and on-fail steps are valid
    /// * `Err(StepError)` - If the referenced task doesn't exist or on-fail steps are invalid
    fn try_from(
        (index, tasks, step_config): (usize, &Tasks, &StepConfig),
    ) -> Result<Self, Self::Error> {
        let on_fail_steps = match step_config.on_fail.as_ref() {
            Some(config) => OnFailSteps::try_from((tasks, config))
                .map_err(StepError::CannotCreateOnFailStepsFromConfig)?,
            None => OnFailSteps::default(),
        };

        Ok(Step {
            index,
            task: tasks.get(&step_config.task).cloned().ok_or_else(|| {
                StepError::CannotCreateTaskFromConfig(step_config.task.to_string())
            })?,
            on_fail_steps,
        })
    }
}

impl Step {
    /// Returns the index of the step in the scenario.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns a reference to the step's task.
    pub fn task(&self) -> &Task {
        &self.task
    }

    /// Returns a reference to the step's on-fail steps.
    pub fn on_fail_steps(&self) -> &OnFailSteps {
        &self.on_fail_steps
    }

    #[instrument(
        name = "step"
        skip_all,
        fields(step.index = self.index)
    )]
    pub(crate) fn execute(
        &self,
        step: &Step,
        session: &Session,
        variables: &Variables,
    ) -> Result<(), StepError> {
        let description = step.task.description().to_string();

        debug!(
            scenario.event = "step_started",
            task.description = description
        );

        let error_message = step.task.error_message().to_string();

        let task_result = match &step.task {
            Task::RemoteSudo { remote_sudo, .. } => remote_sudo
                .execute(session, variables)
                .map_err(|error| {
                    StepError::CannotExecuteRemoteSudoCommand(error, error_message.clone())
                })
                .map_err(|error| {
                    debug!(scenario.event = "error", scenario.error = %error);
                    error
                }),
            Task::SftpCopy { sftp_copy, .. } => sftp_copy
                .execute(session, variables)
                .map_err(|error| {
                    StepError::CannotExecuteSftpCopyCommand(error, error_message.clone())
                })
                .map_err(|error| {
                    debug!(scenario.event = "error", scenario.error = %error);
                    error
                }),
        };

        if let Err(error) = task_result {
            step.execute_on_fail_steps(session, &variables)?;
            return Err(error);
        }

        debug!(scenario.event = "step_completed");
        Ok(())
    }

    fn execute_on_fail_steps(
        &self,
        session: &Session,
        variables: &Variables,
    ) -> Result<(), StepError> {
        self.on_fail_steps
            .execute(session, variables)
            .map_err(StepError::CannotExecuteOnFailSteps)
            .map_err(|error| {
                debug!(scenario.event = "error", scenario.error = %error);
                error
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            on_fail::OnFailStepsConfig,
            step::StepConfig,
            task::{TaskConfig, TaskType},
        },
        scenario::{errors::StepError, step::Step, task::Task, tasks::Tasks},
    };
    use std::collections::HashMap;

    #[test]
    fn test_step_try_from_success_no_on_fail() {
        // Given
        let tasks = create_test_tasks();
        let config = StepConfig {
            task: "task1".to_string(),
            on_fail: None,
        };

        // When
        let result = Step::try_from((0, &tasks, &config));

        // Then
        assert!(result.is_ok());
        let step = result.unwrap();
        assert_eq!(step.task().description(), "Test task 1");
        assert!(step.on_fail_steps().is_empty());
    }

    #[test]
    fn test_step_try_from_success_with_on_fail() {
        // Given
        let tasks = create_test_tasks();
        let config = StepConfig {
            task: "task1".to_string(),
            on_fail: Some(OnFailStepsConfig::from(vec!["task2".to_string()])),
        };

        // When
        let result = Step::try_from((0, &tasks, &config));

        // Then
        assert!(result.is_ok());
        let step = result.unwrap();
        assert_eq!(step.task().description(), "Test task 1");
        assert_eq!(step.on_fail_steps().len(), 1);
    }

    #[test]
    fn test_step_try_from_error_invalid_task() {
        // Given
        let tasks = create_test_tasks();
        let config = StepConfig {
            task: "non_existent_task".to_string(),
            on_fail: None,
        };

        // When
        let result = Step::try_from((0, &tasks, &config));

        // Then
        assert!(result.is_err());
        if let Err(StepError::CannotCreateTaskFromConfig(task_id)) = result {
            assert_eq!(task_id, "non_existent_task");
        } else {
            panic!("Expected CannotCreateTaskFromConfig error");
        }
    }

    #[test]
    fn test_step_try_from_error_invalid_on_fail_task() {
        // Given
        let tasks = create_test_tasks();
        let config = StepConfig {
            task: "task1".to_string(),
            on_fail: Some(OnFailStepsConfig::from(vec![
                "non_existent_task".to_string()
            ])),
        };

        // When
        let result = Step::try_from((0, &tasks, &config));

        // Then
        assert!(result.is_err());
        matches!(result, Err(StepError::CannotCreateOnFailStepsFromConfig(_)));
    }

    #[test]
    fn test_step_accessors() {
        // Given
        let tasks = create_test_tasks();
        let config = StepConfig {
            task: "task1".to_string(),
            on_fail: Some(OnFailStepsConfig::from(vec!["task2".to_string()])),
        };

        // When
        let step = Step::try_from((0, &tasks, &config)).unwrap();

        // Then
        assert_eq!(step.task().description(), "Test task 1");
        assert_eq!(step.on_fail_steps().len(), 1);
    }

    #[test]
    fn test_step_clone() {
        // Given
        let tasks = create_test_tasks();
        let config = StepConfig {
            task: "task1".to_string(),
            on_fail: Some(OnFailStepsConfig::from(vec!["task2".to_string()])),
        };
        let original = Step::try_from((0, &tasks, &config)).unwrap();

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(cloned.task().description(), original.task().description());
        assert_eq!(cloned.on_fail_steps().len(), original.on_fail_steps().len());
    }

    // Test helpers
    fn create_test_tasks() -> Tasks {
        let mut task_map = HashMap::new();
        task_map.insert("task1".to_string(), create_remote_sudo_task());
        task_map.insert("task2".to_string(), create_sftp_copy_task());
        Tasks::from(task_map)
    }

    fn create_remote_sudo_task() -> Task {
        let config = TaskConfig {
            description: "Test task 1".to_string(),
            error_message: "Task 1 failed".to_string(),
            task_type: TaskType::RemoteSudo {
                command: "echo test".to_string(),
            },
        };
        Task::from(&config)
    }

    fn create_sftp_copy_task() -> Task {
        let config = TaskConfig {
            description: "Test task 2".to_string(),
            error_message: "Task 2 failed".to_string(),
            task_type: TaskType::SftpCopy {
                source_path: "/test/source".to_string(),
                destination_path: "/test/dest".to_string(),
            },
        };
        Task::from(&config)
    }
}
