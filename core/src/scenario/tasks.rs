//! Task registry for scenarios.
//!
//! This module provides a registry of tasks that can be referenced and executed
//! as part of scenario steps. It maps task names to their implementations.

use crate::{config::tasks::TasksConfig, scenario::task::Task};
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

impl From<&TasksConfig> for Tasks {
    fn from(config: &TasksConfig) -> Self {
        let mut tasks = HashMap::<String, Task>::new();

        for (id, task_config) in config.deref() {
            let task = Task::from(task_config);
            tasks.insert(id.clone(), task);
        }

        Tasks(tasks)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            task::{TaskConfig, TaskType},
            tasks::TasksConfig,
        },
        scenario::tasks::Tasks,
    };
    use std::collections::HashMap;

    #[test]
    fn test_tasks_from_empty_config() {
        // Given
        let config = TasksConfig::from(HashMap::new());

        // When
        let tasks = Tasks::from(&config);

        // Then
        assert!(tasks.is_empty(), "Tasks should be empty for empty config");
    }

    #[test]
    fn test_tasks_from_config() {
        // Given
        let config = create_test_tasks_config();

        // When
        let tasks = Tasks::from(&config);

        // Then
        assert_eq!(tasks.len(), 2, "Tasks should contain 2 items");
        assert!(tasks.contains_key("task1"));
        assert!(tasks.contains_key("task2"));
    }

    #[test]
    fn test_tasks_deref() {
        // Given
        let config = create_test_tasks_config();
        let tasks = Tasks::from(&config);

        // When & Then
        assert_eq!(tasks.len(), 2, "Should be accessible via Deref");
        assert!(tasks.contains_key("task1"));
    }

    #[test]
    fn test_tasks_deref_mut() {
        // Given
        let config = create_test_tasks_config();
        let mut tasks = Tasks::from(&config);

        // When
        tasks.remove("task1");

        // Then
        assert_eq!(tasks.len(), 1, "Should be mutable via DerefMut");
        assert!(!tasks.contains_key("task1"));
        assert!(tasks.contains_key("task2"));
    }

    #[test]
    fn test_tasks_get_method() {
        // Given
        let config = create_test_tasks_config();
        let tasks = Tasks::from(&config);

        // When
        let task = tasks.get("task1");

        // Then
        assert!(task.is_some());
        assert_eq!(task.unwrap().description(), "Remote command task");
    }

    #[test]
    fn test_tasks_get_nonexistent() {
        // Given
        let config = create_test_tasks_config();
        let tasks = Tasks::from(&config);

        // When
        let task = tasks.get("nonexistent");

        // Then
        assert!(task.is_none());
    }

    fn create_remote_sudo_config() -> TaskConfig {
        TaskConfig {
            description: "Remote command task".to_string(),
            error_message: "Command failed".to_string(),
            task_type: TaskType::RemoteSudo {
                command: "systemctl restart service".to_string(),
            },
        }
    }

    fn create_sftp_copy_config() -> TaskConfig {
        TaskConfig {
            description: "File transfer task".to_string(),
            error_message: "Transfer failed".to_string(),
            task_type: TaskType::SftpCopy {
                source_path: "./local/file".to_string(),
                destination_path: "/remote/file".to_string(),
            },
        }
    }

    fn create_test_tasks_config() -> TasksConfig {
        let mut tasks = HashMap::new();
        tasks.insert("task1".to_string(), create_remote_sudo_config());
        tasks.insert("task2".to_string(), create_sftp_copy_config());
        TasksConfig::from(tasks)
    }
}
