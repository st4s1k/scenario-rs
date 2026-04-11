//! Task registry for scenarios.
//!
//! This module provides a registry of tasks that can be referenced and executed
//! as part of scenario steps. It maps task names to their implementations.

use crate::{
    config::tasks::TasksConfig,
    scenario::{errors::ScenarioError, task::Task},
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// A registry mapping task names to their implementations.
#[derive(Clone, Debug)]
pub struct Tasks(HashMap<String, Task>);

impl Deref for Tasks {
    type Target = HashMap<String, Task>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tasks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<String, Task>> for Tasks {
    fn from(tasks: HashMap<String, Task>) -> Self {
        Tasks(tasks)
    }
}

impl TryFrom<&TasksConfig> for Tasks {
    type Error = ScenarioError;

    fn try_from(config: &TasksConfig) -> Result<Self, Self::Error> {
        let mut tasks = HashMap::<String, Task>::new();

        if let Some(remote_sudo) = &config.remote_sudo {
            for (name, task_config) in remote_sudo {
                if tasks.contains_key(name) {
                    return Err(ScenarioError::DuplicateTaskName(name.clone()));
                }
                tasks.insert(name.clone(), Task::from_remote_sudo(name, task_config));
            }
        }

        if let Some(sftp_copy) = &config.sftp_copy {
            for (name, task_config) in sftp_copy {
                if tasks.contains_key(name) {
                    return Err(ScenarioError::DuplicateTaskName(name.clone()));
                }
                tasks.insert(name.clone(), Task::from_sftp_copy(name, task_config));
            }
        }

        Ok(Tasks(tasks))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            task::{RemoteSudoTaskConfig, SftpCopyTaskConfig},
            tasks::TasksConfig,
        },
        scenario::tasks::Tasks,
    };
    use std::collections::HashMap;

    #[test]
    fn test_tasks_from_empty_config() {
        // Given
        let config = TasksConfig::default();

        // When
        let tasks = Tasks::try_from(&config).unwrap();

        // Then
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_tasks_from_config() {
        // Given
        let config = create_test_tasks_config();

        // When
        let tasks = Tasks::try_from(&config).unwrap();

        // Then
        assert_eq!(tasks.len(), 2);
        assert!(tasks.contains_key("restart"));
        assert!(tasks.contains_key("deploy_config"));
    }

    #[test]
    fn test_tasks_descriptions_default_to_name() {
        // Given
        let config = TasksConfig {
            remote_sudo: Some(HashMap::from([(
                "restart".to_string(),
                RemoteSudoTaskConfig {
                    command: "systemctl restart app".to_string(),
                    description: None,
                    error_message: None,
                },
            )])),
            sftp_copy: None,
        };

        // When
        let tasks = Tasks::try_from(&config).unwrap();

        // Then
        assert_eq!(tasks.get("restart").unwrap().description(), "restart");
        assert_eq!(
            tasks.get("restart").unwrap().error_message(),
            "Remote command failed"
        );
    }

    #[test]
    fn test_tasks_duplicate_name_across_categories() {
        // Given
        let config = TasksConfig {
            remote_sudo: Some(HashMap::from([(
                "deploy".to_string(),
                RemoteSudoTaskConfig {
                    command: "echo deploy".to_string(),
                    description: None,
                    error_message: None,
                },
            )])),
            sftp_copy: Some(HashMap::from([(
                "deploy".to_string(),
                SftpCopyTaskConfig {
                    source: "/src".to_string(),
                    destination: "/dst".to_string(),
                    description: None,
                    error_message: None,
                },
            )])),
        };

        // When
        let result = Tasks::try_from(&config);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_tasks_get_nonexistent() {
        // Given
        let config = create_test_tasks_config();
        let tasks = Tasks::try_from(&config).unwrap();

        // When & Then
        assert!(tasks.get("nonexistent").is_none());
    }

    #[test]
    fn test_tasks_deref_mut() {
        // Given
        let config = create_test_tasks_config();
        let mut tasks = Tasks::try_from(&config).unwrap();

        // When
        tasks.remove("restart");

        // Then
        assert!(!tasks.contains_key("restart"));
    }

    #[test]
    fn test_tasks_duplicate_name_in_sftp_copy_only() {
        // Given — insert a task via remote_sudo, then same name in sftp_copy
        // This ensures the DuplicateTaskName error is hit in the sftp_copy loop (line 48)
        let config = TasksConfig {
            remote_sudo: Some(HashMap::from([(
                "upload".to_string(),
                RemoteSudoTaskConfig {
                    command: "echo upload".to_string(),
                    description: None,
                    error_message: None,
                },
            )])),
            sftp_copy: Some(HashMap::from([(
                "upload".to_string(),
                SftpCopyTaskConfig {
                    source: "/src".to_string(),
                    destination: "/dst".to_string(),
                    description: None,
                    error_message: None,
                },
            )])),
        };

        // When
        let result = Tasks::try_from(&config);

        // Then
        assert!(result.is_err());
    }

    fn create_test_tasks_config() -> TasksConfig {
        TasksConfig {
            remote_sudo: Some(HashMap::from([(
                "restart".to_string(),
                RemoteSudoTaskConfig {
                    command: "systemctl restart service".to_string(),
                    description: Some("Remote command task".to_string()),
                    error_message: Some("Command failed".to_string()),
                },
            )])),
            sftp_copy: Some(HashMap::from([(
                "deploy_config".to_string(),
                SftpCopyTaskConfig {
                    source: "./local/file".to_string(),
                    destination: "/remote/file".to_string(),
                    description: Some("File transfer task".to_string()),
                    error_message: Some("Transfer failed".to_string()),
                },
            )])),
        }
    }
}
