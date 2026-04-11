//! Step execution management for scenarios.
//!
//! This module provides functionality for executing a sequence of steps
//! in a scenario, handling failures and executing fallback steps when needed.

use crate::{
    config::{on_fail::OnFailStepsConfig, sequences::SequencesConfig, steps::StepsConfig},
    scenario::{
        errors::{ScenarioError, StepsError},
        on_fail_steps::OnFailSteps,
        step::Step,
        tasks::Tasks,
        variables::Variables,
    },
    session::Session,
    state::ExecutionStateManager,
    trace::ScenarioEvent,
};
use std::ops::{Deref, DerefMut};
use tracing::{debug, instrument};

/// An ordered collection of steps defining a scenario's execution flow.
#[derive(Clone, Debug)]
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

impl Steps {
    /// Build Steps from the new config types: ordered StepsConfig + SequencesConfig + Tasks.
    pub fn from_config(
        tasks: &Tasks,
        steps_config: &StepsConfig,
        sequences_config: &SequencesConfig,
    ) -> Result<Self, ScenarioError> {
        let mut steps = Vec::new();
        let mut index = 0;

        for (step_name, step_context) in steps_config.iter() {
            match (&step_context.task, &step_context.sequence) {
                (Some(task_name), None) => {
                    // Single task step
                    let task = tasks
                        .get(task_name)
                        .cloned()
                        .ok_or_else(|| ScenarioError::UnknownTaskReference(task_name.clone()))?;

                    let on_fail_steps = Self::resolve_on_fail(
                        tasks,
                        sequences_config,
                        step_context.on_fail.as_deref(),
                        step_name,
                    )?;

                    steps.push(Step::new(index, task, on_fail_steps));
                    index += 1;
                }
                (None, Some(seq_name)) => {
                    // Sequence step: expand to multiple steps
                    let task_names = sequences_config.get(seq_name).ok_or_else(|| {
                        ScenarioError::UnknownSequence(seq_name.clone(), step_name.clone())
                    })?;

                    let on_fail_steps = Self::resolve_on_fail(
                        tasks,
                        sequences_config,
                        step_context.on_fail.as_deref(),
                        step_name,
                    )?;

                    for task_name in task_names {
                        let task = tasks.get(task_name).cloned().ok_or_else(|| {
                            ScenarioError::UnknownTaskReference(task_name.clone())
                        })?;
                        steps.push(Step::new(index, task, on_fail_steps.clone()));
                        index += 1;
                    }
                }
                _ => {
                    return Err(ScenarioError::InvalidStepContext(step_name.clone()));
                }
            }
        }

        Ok(Steps(steps))
    }

    /// Resolve an on-fail sequence reference into runtime OnFailSteps.
    fn resolve_on_fail(
        tasks: &Tasks,
        sequences_config: &SequencesConfig,
        on_fail_ref: Option<&str>,
        step_name: &str,
    ) -> Result<OnFailSteps, ScenarioError> {
        match on_fail_ref {
            Some(seq_name) => {
                let task_names = sequences_config.get(seq_name).ok_or_else(|| {
                    ScenarioError::UnknownSequence(seq_name.to_string(), step_name.to_string())
                })?;
                // Validate all task names exist
                for task_name in task_names {
                    if !tasks.contains_key(task_name) {
                        return Err(ScenarioError::UnknownTaskReference(task_name.clone()));
                    }
                }
                let on_fail_config = OnFailStepsConfig::from(task_names.clone());
                OnFailSteps::try_from((tasks, &on_fail_config))
                    .map_err(|e| ScenarioError::UnknownTaskReference(e.to_string()))
            }
            None => Ok(OnFailSteps::default()),
        }
    }
}

impl From<Vec<Step>> for Steps {
    fn from(steps: Vec<Step>) -> Self {
        Steps(steps)
    }
}

impl Default for Steps {
    fn default() -> Self {
        Steps(Vec::new())
    }
}

impl Steps {
    /// Executes all steps in sequence. On failure, runs on-fail steps then stops.
    #[instrument(
        name = "steps",
        skip_all,
        fields(steps.total = self.len())
    )]
    pub fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        state_manager: Option<&ExecutionStateManager>,
    ) -> Result<(), StepsError> {
        if self.is_empty() {
            return Ok(());
        }

        debug!(scenario.event = ScenarioEvent::StepsStarted.as_str());

        for step in self.iter() {
            step.execute(session, variables, state_manager)
                .map_err(StepsError::CannotExecuteStep)?;
        }

        debug!(scenario.event = ScenarioEvent::StepsCompleted.as_str());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            sequences::SequencesConfig,
            step::StepContext,
            steps::StepsConfig,
            task::{RemoteSudoTaskConfig, SftpCopyTaskConfig},
            tasks::TasksConfig,
        },
        scenario::{
            errors::ScenarioError,
            on_fail_steps::OnFailSteps,
            step::Step,
            steps::Steps,
            task::Task,
            tasks::Tasks,
        },
    };
    use indexmap::IndexMap;
    use std::collections::HashMap;

    fn create_test_tasks() -> Tasks {
        let config = TasksConfig {
            remote_sudo: Some(HashMap::from([
                (
                    "task1".to_string(),
                    RemoteSudoTaskConfig {
                        command: "echo test1".to_string(),
                        description: Some("Test task 1".to_string()),
                        error_message: None,
                    },
                ),
                (
                    "task3".to_string(),
                    RemoteSudoTaskConfig {
                        command: "echo test3".to_string(),
                        description: Some("Test task 3".to_string()),
                        error_message: None,
                    },
                ),
            ])),
            sftp_copy: Some(HashMap::from([(
                "task2".to_string(),
                SftpCopyTaskConfig {
                    source: "/test/source".to_string(),
                    destination: "/test/dest".to_string(),
                    description: Some("Test task 2".to_string()),
                    error_message: None,
                },
            )])),
        };
        Tasks::try_from(&config).unwrap()
    }

    fn create_steps_config(entries: Vec<(&str, StepContext)>) -> StepsConfig {
        let map: IndexMap<String, StepContext> = entries
            .into_iter()
            .map(|(name, ctx)| (name.to_string(), ctx))
            .collect();
        StepsConfig::from(map)
    }

    #[test]
    fn test_steps_from_config_single_tasks() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![
            (
                "step_one",
                StepContext {
                    task: Some("task1".to_string()),
                    sequence: None,
                    on_fail: None,
                },
            ),
            (
                "step_two",
                StepContext {
                    task: Some("task2".to_string()),
                    sequence: None,
                    on_fail: None,
                },
            ),
        ]);
        let sequences = SequencesConfig::default();

        // When
        let steps = Steps::from_config(&tasks, &steps_config, &sequences).unwrap();

        // Then
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].task().description(), "Test task 1");
        assert_eq!(steps[1].task().description(), "Test task 2");
        assert_eq!(steps[0].index(), 0);
        assert_eq!(steps[1].index(), 1);
    }

    #[test]
    fn test_steps_from_config_sequence_expansion() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![(
            "deploy",
            StepContext {
                task: None,
                sequence: Some("deploy_seq".to_string()),
                on_fail: None,
            },
        )]);
        let sequences = SequencesConfig::from(HashMap::from([(
            "deploy_seq".to_string(),
            vec!["task1".to_string(), "task2".to_string()],
        )]));

        // When
        let steps = Steps::from_config(&tasks, &steps_config, &sequences).unwrap();

        // Then
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].index(), 0);
        assert_eq!(steps[1].index(), 1);
        assert_eq!(steps[0].task().description(), "Test task 1");
        assert_eq!(steps[1].task().description(), "Test task 2");
    }

    #[test]
    fn test_steps_from_config_with_on_fail() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![(
            "risky_step",
            StepContext {
                task: Some("task1".to_string()),
                sequence: None,
                on_fail: Some("cleanup".to_string()),
            },
        )]);
        let sequences = SequencesConfig::from(HashMap::from([(
            "cleanup".to_string(),
            vec!["task3".to_string()],
        )]));

        // When
        let steps = Steps::from_config(&tasks, &steps_config, &sequences).unwrap();

        // Then
        assert_eq!(steps.len(), 1);
        assert!(!steps[0].on_fail_steps.is_empty());
    }

    #[test]
    fn test_steps_from_config_unknown_task() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![(
            "bad_step",
            StepContext {
                task: Some("nonexistent".to_string()),
                sequence: None,
                on_fail: None,
            },
        )]);
        let sequences = SequencesConfig::default();

        // When
        let result = Steps::from_config(&tasks, &steps_config, &sequences);

        // Then
        assert!(matches!(result, Err(ScenarioError::UnknownTaskReference(_))));
    }

    #[test]
    fn test_steps_from_config_unknown_sequence() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![(
            "bad_step",
            StepContext {
                task: None,
                sequence: Some("nonexistent".to_string()),
                on_fail: None,
            },
        )]);
        let sequences = SequencesConfig::default();

        // When
        let result = Steps::from_config(&tasks, &steps_config, &sequences);

        // Then
        assert!(matches!(result, Err(ScenarioError::UnknownSequence(_, _))));
    }

    #[test]
    fn test_steps_from_config_invalid_context() {
        // Given
        let tasks = create_test_tasks();
        // Neither task nor sequence set
        let steps_config = create_steps_config(vec![(
            "bad_step",
            StepContext {
                task: None,
                sequence: None,
                on_fail: None,
            },
        )]);
        let sequences = SequencesConfig::default();

        // When
        let result = Steps::from_config(&tasks, &steps_config, &sequences);

        // Then
        assert!(matches!(result, Err(ScenarioError::InvalidStepContext(_))));
    }

    #[test]
    fn test_steps_from_config_both_task_and_sequence() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![(
            "bad_step",
            StepContext {
                task: Some("task1".to_string()),
                sequence: Some("some_seq".to_string()),
                on_fail: None,
            },
        )]);
        let sequences = SequencesConfig::default();

        // When
        let result = Steps::from_config(&tasks, &steps_config, &sequences);

        // Then
        assert!(matches!(result, Err(ScenarioError::InvalidStepContext(_))));
    }

    #[test]
    fn test_steps_from_config_unknown_on_fail_sequence() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![(
            "step",
            StepContext {
                task: Some("task1".to_string()),
                sequence: None,
                on_fail: Some("nonexistent_cleanup".to_string()),
            },
        )]);
        let sequences = SequencesConfig::default();

        // When
        let result = Steps::from_config(&tasks, &steps_config, &sequences);

        // Then
        assert!(matches!(result, Err(ScenarioError::UnknownSequence(_, _))));
    }

    #[test]
    fn test_steps_from_config_empty() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = StepsConfig::default();
        let sequences = SequencesConfig::default();

        // When
        let steps = Steps::from_config(&tasks, &steps_config, &sequences).unwrap();

        // Then
        assert!(steps.is_empty());
    }

    #[test]
    fn test_steps_default() {
        // Given & When
        let steps = Steps::default();

        // Then
        assert!(steps.is_empty());
    }

    #[test]
    fn test_steps_from_vec() {
        // Given
        let task = Task::from_remote_sudo(
            "t",
            &RemoteSudoTaskConfig {
                command: "echo hi".to_string(),
                description: None,
                error_message: None,
            },
        );

        // When
        let steps = Steps::from(vec![Step::new(0, task, OnFailSteps::default())]);

        // Then
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_steps_deref() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![
            (
                "a",
                StepContext {
                    task: Some("task1".to_string()),
                    sequence: None,
                    on_fail: None,
                },
            ),
            (
                "b",
                StepContext {
                    task: Some("task2".to_string()),
                    sequence: None,
                    on_fail: None,
                },
            ),
        ]);
        let sequences = SequencesConfig::default();

        // When
        let steps = Steps::from_config(&tasks, &steps_config, &sequences).unwrap();

        // Then
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].task().description(), "Test task 1");
        assert_eq!(steps[1].task().description(), "Test task 2");
    }

    #[test]
    fn test_steps_deref_mut() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![
            (
                "a",
                StepContext {
                    task: Some("task1".to_string()),
                    sequence: None,
                    on_fail: None,
                },
            ),
            (
                "b",
                StepContext {
                    task: Some("task2".to_string()),
                    sequence: None,
                    on_fail: None,
                },
            ),
        ]);
        let sequences = SequencesConfig::default();

        let mut steps = Steps::from_config(&tasks, &steps_config, &sequences).unwrap();

        // When
        steps.pop();

        // Then
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_steps_clone() {
        // Given
        let tasks = create_test_tasks();
        let steps_config = create_steps_config(vec![
            (
                "a",
                StepContext {
                    task: Some("task1".to_string()),
                    sequence: None,
                    on_fail: None,
                },
            ),
            (
                "b",
                StepContext {
                    task: Some("task2".to_string()),
                    sequence: None,
                    on_fail: None,
                },
            ),
        ]);
        let sequences = SequencesConfig::default();

        let original = Steps::from_config(&tasks, &steps_config, &sequences).unwrap();

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(cloned.len(), original.len());
        assert_eq!(
            cloned[0].task().description(),
            original[0].task().description()
        );
    }

    #[test]
    fn test_steps_execute_non_empty_success() {
        use crate::{
            scenario::variables::Variables,
            session::{Channel, Session, SessionType, Sftp, SshError, Write},
            utils::{ArcMutex, Wrap},
        };

        struct TestChannel;
        impl Channel for TestChannel {
            fn exec(&mut self, _: &str) -> Result<(), SshError> { Ok(()) }
            fn read_to_string(&mut self, _: &mut String) -> Result<usize, SshError> { Ok(0) }
            fn exit_status(&self) -> Result<i32, SshError> { Ok(0) }
        }
        struct TestWrite;
        impl Write for TestWrite {
            fn write_all(&mut self, _: &[u8]) -> Result<(), SshError> { Ok(()) }
        }
        struct TestSftp;
        impl Sftp for TestSftp {
            fn create(&self, _: &std::path::Path) -> Result<Box<dyn Write>, SshError> {
                Ok(Box::new(TestWrite))
            }
        }

        // Given
        let task = Task::from_remote_sudo(
            "t",
            &RemoteSudoTaskConfig {
                command: "echo hi".to_string(),
                description: None,
                error_message: None,
            },
        );
        let steps = Steps::from(vec![Step::new(0, task, OnFailSteps::default())]);
        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(TestChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };
        let variables = Variables::default();

        // When
        let result = steps.execute(&session, &variables, None);

        // Then
        assert!(result.is_ok());
    }
}
