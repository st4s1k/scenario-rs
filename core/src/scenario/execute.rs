use crate::{
    config::ExecuteConfig,
    scenario::{errors::ExecuteError, steps::Steps, tasks::Tasks},
};

#[derive(Clone, Debug)]
pub struct Execute {
    pub(crate) steps: Steps,
}

impl Default for Execute {
    fn default() -> Self {
        Execute {
            steps: Steps::default(),
        }
    }
}

impl TryFrom<(&Tasks, &ExecuteConfig)> for Execute {
    type Error = ExecuteError;

    fn try_from((tasks, config): (&Tasks, &ExecuteConfig)) -> Result<Self, Self::Error> {
        let steps = Steps::try_from((tasks, &config.steps))
            .map_err(ExecuteError::CannotCreateStepsFromConfig)?;
        Ok(Execute { steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{StepConfig, StepsConfig, TaskConfig};
    use crate::scenario::errors::ExecuteError;
    use crate::scenario::task::Task;
    use std::collections::HashMap;

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
            assert!(
                error_string.contains("non_existent_task"),
                "Error message should contain the invalid task name: {}",
                error_string
            );
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
        assert_eq!(execute.steps.len(), 0, "Execute should contain 0 steps");
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

    fn create_test_tasks() -> Tasks {
        let mut task_map = HashMap::new();
        task_map.insert("task1".to_string(), create_remote_sudo_task());
        task_map.insert("task2".to_string(), create_sftp_copy_task());
        Tasks(task_map)
    }

    fn create_remote_sudo_task() -> Task {
        let config = TaskConfig::RemoteSudo {
            description: "Test task 1".to_string(),
            error_message: "Task 1 failed".to_string(),
            remote_sudo: crate::config::RemoteSudoConfig {
                command: "echo test".to_string(),
            },
        };
        Task::from(&config)
    }

    fn create_sftp_copy_task() -> Task {
        let config = TaskConfig::SftpCopy {
            description: "Test task 2".to_string(),
            error_message: "Task 2 failed".to_string(),
            sftp_copy: crate::config::SftpCopyConfig {
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
