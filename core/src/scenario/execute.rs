use crate::{
    config::execute::ExecuteConfig,
    scenario::{errors::ExecuteError, steps::Steps, tasks::Tasks},
};

/// Represents the executable part of a scenario, containing ordered steps to be executed.
///
/// An `Execute` instance holds the sequence of steps that define the execution flow
/// of a scenario.
#[derive(Clone, Debug)]
pub struct Execute {
    pub(crate) steps: Steps,
}

impl Default for Execute {
    /// Creates a default `Execute` instance with no steps.
    fn default() -> Self {
        Execute {
            steps: Steps::default(),
        }
    }
}

impl TryFrom<(&Tasks, &ExecuteConfig)> for Execute {
    type Error = ExecuteError;

    /// Attempts to create an `Execute` instance from a combination of tasks and execution configuration.
    ///
    /// This conversion will validate that all task references in the configuration exist in the provided tasks.
    ///
    /// # Errors
    ///
    /// Returns an `ExecuteError::CannotCreateStepsFromConfig` if:
    /// - A task referenced in the configuration doesn't exist in the tasks collection
    /// - Other validation errors occur during steps creation
    fn try_from((tasks, config): (&Tasks, &ExecuteConfig)) -> Result<Self, Self::Error> {
        let steps = Steps::try_from((tasks, &config.steps))
            .map_err(ExecuteError::CannotCreateStepsFromConfig)?;
        Ok(Execute { steps })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            step::StepConfig,
            steps::StepsConfig,
            task::{TaskConfig, TaskType},
        },
        scenario::task::Task,
    };

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_execute_default() {
        // Given & When
        let execute = Execute::default();

        // Then
        assert!(
            execute.steps.is_empty(),
            "Default Execute should have no steps"
        );
    }

    #[test]
    fn test_execute_try_from_success() {
        // Given
        let tasks = create_test_tasks();
        let config = create_valid_execute_config();

        // When
        let result = Execute::try_from((&tasks, &config));

        // Then
        assert!(
            result.is_ok(),
            "Execute::try_from should succeed with valid input"
        );
        assert_eq!(
            result.unwrap().steps.len(),
            2,
            "Execute should contain 2 steps"
        );
    }

    #[test]
    fn test_execute_try_from_error() {
        // Given
        let tasks = create_test_tasks();
        let config = create_invalid_execute_config();

        // When
        let result = Execute::try_from((&tasks, &config));

        // Then
        assert!(
            result.is_err(),
            "Execute::try_from should fail with invalid input"
        );
        if let Err(ExecuteError::CannotCreateStepsFromConfig(err_msg)) = result {
            let error_string = format!("{:?}", err_msg);
            assert!(error_string.contains("non_existent_task"));
        }
    }

    #[test]
    fn test_execute_try_from_empty_steps() {
        // Given
        let tasks = create_test_tasks();
        let config = ExecuteConfig {
            steps: StepsConfig(vec![]),
        };

        // When
        let result = Execute::try_from((&tasks, &config));

        // Then
        assert!(
            result.is_ok(),
            "Execute::try_from should succeed with empty steps"
        );
        let execute = result.unwrap();
        assert!(execute.steps.is_empty(), "Execute should contain no steps");
    }

    #[test]
    fn test_execute_try_from_with_duplicated_tasks() {
        // Given
        let tasks = create_test_tasks();
        let config = ExecuteConfig {
            steps: StepsConfig(vec![
                StepConfig {
                    task: "task1".to_string(),
                    on_fail: None,
                },
                StepConfig {
                    task: "task1".to_string(),
                    on_fail: None,
                },
            ]),
        };

        // When
        let result = Execute::try_from((&tasks, &config));

        // Then
        assert!(
            result.is_ok(),
            "Execute::try_from should allow duplicated task references"
        );
        let execute = result.unwrap();
        assert_eq!(execute.steps.len(), 2, "Execute should contain 2 steps");
    }

    #[test]
    fn test_execute_try_from_empty_tasks() {
        // Given
        let tasks = Tasks(HashMap::new());
        let config = create_valid_execute_config();

        // When
        let result = Execute::try_from((&tasks, &config));

        // Then
        assert!(
            result.is_err(),
            "Execute::try_from should fail with empty tasks"
        );
        if let Err(ExecuteError::CannotCreateStepsFromConfig(err_msg)) = result {
            let error_string = format!("{:?}", err_msg);
            assert!(error_string.contains("task1"));
        }
    }

    fn create_test_tasks() -> Tasks {
        let mut task_map = HashMap::new();
        task_map.insert("task1".to_string(), create_remote_sudo_task());
        task_map.insert("task2".to_string(), create_sftp_copy_task());
        Tasks(task_map)
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

    fn create_valid_execute_config() -> ExecuteConfig {
        ExecuteConfig {
            steps: StepsConfig(vec![
                StepConfig {
                    task: "task1".to_string(),
                    on_fail: None,
                },
                StepConfig {
                    task: "task2".to_string(),
                    on_fail: None,
                },
            ]),
        }
    }

    fn create_invalid_execute_config() -> ExecuteConfig {
        ExecuteConfig {
            steps: StepsConfig(vec![StepConfig {
                task: "non_existent_task".to_string(),
                on_fail: None,
            }]),
        }
    }
}
