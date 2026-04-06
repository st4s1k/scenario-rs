//! Step execution handling for scenarios.
//!
//! This module provides functionality for executing individual steps
//! in a scenario, including regular tasks and fallback steps for error handling.

use crate::{
    config::step::StepConfig,
    scenario::{
        errors::StepError, on_fail_steps::OnFailSteps, task::Task, tasks::Tasks, variables::Variables,
    },
    session::Session,
    state::{ExecutionStateManager, TaskTracker},
    state::types::StepStatus,
    trace::ScenarioEvent,
};
use tracing::{debug, instrument};

/// A single step in a scenario: a task with optional on-fail recovery steps.
#[derive(Clone, Debug)]
pub struct Step {
    pub index: usize,
    pub task: Task,
    pub on_fail_steps: OnFailSteps,
}

impl TryFrom<(usize, &Tasks, &StepConfig)> for Step {
    type Error = StepError;

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
        session: &Session,
        variables: &Variables,
        state_manager: Option<&ExecutionStateManager>,
    ) -> Result<(), StepError> {
        let description = self.task.description().to_string();

        debug!(
            scenario.event = ScenarioEvent::StepStarted.as_str(),
            task.description = description
        );

        if let Some(sm) = state_manager {
            sm.update_step_status(self.index, StepStatus::Running);
        }

        let error_message = self.task.error_message().to_string();
        let tracker = state_manager.map(|sm| TaskTracker::for_step(sm, self.index));

        let task_result = match &self.task {
            Task::RemoteSudo { remote_sudo, .. } => remote_sudo
                .execute(session, variables, tracker.as_ref())
                .map_err(|error| {
                    StepError::CannotExecuteRemoteSudoCommand(error, error_message.clone())
                })
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                }),
            Task::SftpCopy { sftp_copy, .. } => sftp_copy
                .execute(session, variables, tracker.as_ref())
                .map_err(|error| {
                    StepError::CannotExecuteSftpCopyCommand(error, error_message.clone())
                })
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                }),
        };

        if let Err(error) = task_result {
            if let Some(sm) = state_manager {
                sm.update_step_status(self.index, StepStatus::Failed);
                sm.add_step_error(self.index, error.to_string());
            }
            self.execute_on_fail_steps(session, &variables, state_manager)?;
            return Err(error);
        }

        debug!(scenario.event = ScenarioEvent::StepCompleted.as_str());
        if let Some(sm) = state_manager {
            sm.update_step_status(self.index, StepStatus::Completed);
        }
        Ok(())
    }

    fn execute_on_fail_steps(
        &self,
        session: &Session,
        variables: &Variables,
        state_manager: Option<&ExecutionStateManager>,
    ) -> Result<(), StepError> {
        self.on_fail_steps
            .execute(session, variables, state_manager, self.index)
            .map_err(StepError::CannotExecuteOnFailSteps)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
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
