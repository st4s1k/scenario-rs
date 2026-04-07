use crate::{
    config::on_fail::OnFailStepsConfig,
    scenario::{
        errors::OnFailError, on_fail_step::OnFailStep, task::Task, tasks::Tasks,
        variables::Variables,
    },
    session::Session,
    state::ExecutionStateManager,
    trace::ScenarioEvent,
};
use std::ops::{Deref, DerefMut};
use tracing::{debug, instrument};

/// Recovery steps executed when a scenario step fails.
#[derive(Clone, Debug)]
pub struct OnFailSteps(Vec<OnFailStep>);

impl Deref for OnFailSteps {
    type Target = Vec<OnFailStep>;

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
        let mut on_fail_steps: Vec<OnFailStep> = Vec::new();
        for (index, config_step) in config.deref().iter().enumerate() {
            let task: Task = tasks
                .get(config_step)
                .cloned()
                .ok_or_else(|| OnFailError::InvalidOnFailStep(config_step.clone()))
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                })?;
            let on_fail_step = OnFailStep::from((index, task));
            on_fail_steps.push(on_fail_step);
        }
        Ok(OnFailSteps(on_fail_steps))
    }
}

impl From<Vec<OnFailStep>> for OnFailSteps {
    fn from(steps: Vec<OnFailStep>) -> Self {
        OnFailSteps(steps)
    }
}

impl Default for OnFailSteps {
    fn default() -> Self {
        OnFailSteps(Vec::new())
    }
}

impl OnFailSteps {
    /// Executes all on-fail tasks in sequence.
    #[instrument(
        name = "on_fail_steps",
        skip_all,
        fields(on_fail_steps.total = self.len())
    )]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        state_manager: Option<&ExecutionStateManager>,
        parent_step_index: usize,
    ) -> Result<(), OnFailError> {
        if self.is_empty() {
            return Ok(());
        }

        debug!(scenario.event = ScenarioEvent::OnFailStepsStarted.as_str());

        for step in self.iter() {
            step.execute(session, variables, state_manager, parent_step_index)?;
        }

        debug!(scenario.event = ScenarioEvent::OnFailStepsCompleted.as_str());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            on_fail::OnFailStepsConfig,
            task::{TaskConfig, TaskType},
        },
        scenario::{
            on_fail_step::OnFailStep,
            on_fail_steps::{OnFailError, OnFailSteps},
            sftp_copy::SftpCopy,
            task::Task,
            tasks::Tasks,
            variables::Variables,
        },
       session::Session,
    };
    use std::collections::HashMap;

    #[test]
    fn test_on_fail_steps_default() {
        // Given & When
        let on_fail_steps = OnFailSteps::default();

        // Then
        assert!(
            on_fail_steps.is_empty(),
            "Default OnFailSteps should be empty"
        );
    }

    #[test]
    fn test_on_fail_steps_try_from_success() {
        // Given
        let tasks = create_test_tasks();
        let config = create_valid_on_fail_config();

        // When
        let result = OnFailSteps::try_from((&tasks, &config));

        // Then
        assert!(
            result.is_ok(),
            "OnFailSteps::try_from should succeed with valid input"
        );
        assert_eq!(
            result.unwrap().len(),
            2,
            "OnFailSteps should contain 2 steps"
        );
    }

    #[test]
    fn test_on_fail_steps_try_from_error() {
        // Given
        let tasks = create_test_tasks();
        let config = create_invalid_on_fail_config();

        // When
        let result = OnFailSteps::try_from((&tasks, &config));

        // Then
        assert!(
            result.is_err(),
            "OnFailSteps::try_from should fail with invalid input"
        );
        if let Err(OnFailError::InvalidOnFailStep(invalid_step)) = result {
            assert_eq!(
                invalid_step, "non_existent_task",
                "Error should contain the invalid task name"
            );
        } else {
            panic!("Expected InvalidOnFailStep error");
        }
    }

    #[test]
    fn test_on_fail_steps_deref() {
        // Given
        let vec = vec![create_remote_sudo_step()];
        let on_fail_steps = OnFailSteps(vec.clone());

        // When
        let task_description = on_fail_steps[0].task.description();

        // Then
        assert_eq!(on_fail_steps.len(), 1);
        assert_eq!(
            task_description, "Test task 1",
            "Should be able to access elements through Deref"
        );
    }

    #[test]
    fn test_on_fail_steps_deref_mut() {
        // Given
        let mut on_fail_steps = OnFailSteps(vec![create_remote_sudo_step()]);

        // When
        on_fail_steps.push(create_sftp_copy_step());

        // Then
        assert_eq!(
            on_fail_steps.len(),
            2,
            "Should be able to modify through DerefMut"
        );
        assert_eq!(on_fail_steps[1].task.description(), "Test task 2");
    }

    #[test]
    fn test_on_fail_steps_try_from_empty_config() {
        // Given
        let tasks = create_test_tasks();
        let config = OnFailStepsConfig::default();

        // When
        let result = OnFailSteps::try_from((&tasks, &config));

        // Then
        assert!(
            result.is_ok(),
            "OnFailSteps::try_from should succeed with empty config"
        );
        assert!(
            result.unwrap().is_empty(),
            "OnFailSteps should be empty with empty config"
        );
    }

    #[test]
    fn test_on_fail_steps_execute_sftp_copy_error() {
        // Given
        let sftp_copy = SftpCopy {
            source_path: "{non-existent-var}".to_string(),
            destination_path: "/test/dest".to_string(),
        };

        let failing_task = Task::SftpCopy {
            sftp_copy,
            description: "Failing sftp copy".to_string(),
            error_message: "Failed".to_string(),
        };

        let on_fail_steps = OnFailSteps(vec![OnFailStep {
            index: 0,
            task: failing_task,
        }]);
        let session = Session::default();
        let variables = Variables::default();

        // When
        let result = on_fail_steps.execute(&session, &variables, None, 0);

        // Then
        assert!(result.is_err(), "Execute should fail with sftp copy error");
        if let Err(OnFailError::CannotOnFailSftpCopy(_)) = result {
        } else {
            panic!("Expected CannotOnFailSftpCopy error");
        }
    }

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

    fn create_remote_sudo_step() -> OnFailStep {
        OnFailStep {
            index: 0,
            task: create_remote_sudo_task(),
        }
    }

    fn create_sftp_copy_step() -> OnFailStep {
        OnFailStep {
            index: 0,
            task: create_sftp_copy_task(),
        }
    }

    fn create_valid_on_fail_config() -> OnFailStepsConfig {
        OnFailStepsConfig::from(vec!["task1".to_string(), "task2".to_string()])
    }

    fn create_invalid_on_fail_config() -> OnFailStepsConfig {
        OnFailStepsConfig::from(vec!["non_existent_task".to_string()])
    }
}
