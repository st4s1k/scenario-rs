use crate::config::task::TaskConfig;
use serde::Deserialize;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// Configuration for all tasks available in a scenario.
///
/// This struct represents a collection of task configurations, where each task
/// is identified by a unique name. These tasks can be referenced in steps to
/// define the execution flow of a scenario.
///
/// Tasks are stored in a HashMap where the key is the task name and the value
/// is its configuration.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct TasksConfig(HashMap<String, TaskConfig>);

impl Deref for TasksConfig {
    type Target = HashMap<String, TaskConfig>;

    /// Dereferences to the underlying HashMap of task configurations.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TasksConfig {
    /// Provides mutable access to the underlying HashMap of task configurations.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<String, TaskConfig>> for TasksConfig {
    /// Creates a TasksConfig from a HashMap of task configurations.
    ///
    /// This constructor allows for the creation of a TasksConfig from an existing
    /// HashMap, enabling flexibility in how task configurations are initialized.
    ///
    /// # Arguments
    ///
    /// * `tasks` - A HashMap containing task names and their configurations
    fn from(tasks: HashMap<String, TaskConfig>) -> Self {
        TasksConfig(tasks)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{
        task::{TaskConfig, TaskType},
        tasks::TasksConfig,
    };
    use std::collections::HashMap;

    #[test]
    fn test_tasks_config_default() {
        // Given & When
        let tasks = TasksConfig::default();

        // Then
        assert!(tasks.is_empty(), "Default TasksConfig should be empty");
    }

    #[test]
    fn test_tasks_config_from_hashmap() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "task1".to_string(),
            TaskConfig {
                description: "Test task".to_string(),
                error_message: "Test failed".to_string(),
                task_type: TaskType::RemoteSudo {
                    command: "echo test".to_string(),
                },
            },
        );

        // When
        let tasks = TasksConfig::from(map);

        // Then
        assert_eq!(tasks.len(), 1);
        assert!(tasks.contains_key("task1"));
    }

    #[test]
    fn test_tasks_config_deref() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "task1".to_string(),
            TaskConfig {
                description: "Test task".to_string(),
                error_message: "Test failed".to_string(),
                task_type: TaskType::RemoteSudo {
                    command: "echo test".to_string(),
                },
            },
        );
        let tasks = TasksConfig::from(map);

        // When & Then
        assert_eq!(tasks.len(), 1);
        assert!(tasks.contains_key("task1"));
        let task = tasks.get("task1").unwrap();
        assert_eq!(task.description, "Test task");
    }

    #[test]
    fn test_tasks_config_deref_mut() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "task1".to_string(),
            TaskConfig {
                description: "Test task".to_string(),
                error_message: "Test failed".to_string(),
                task_type: TaskType::RemoteSudo {
                    command: "echo test".to_string(),
                },
            },
        );
        let mut tasks = TasksConfig::from(map);

        // When
        tasks.insert(
            "task2".to_string(),
            TaskConfig {
                description: "Another task".to_string(),
                error_message: "Another failed".to_string(),
                task_type: TaskType::SftpCopy {
                    source_path: "/source".to_string(),
                    destination_path: "/dest".to_string(),
                },
            },
        );
        tasks.remove("task1");

        // Then
        assert_eq!(tasks.len(), 1);
        assert!(tasks.contains_key("task2"));
        assert!(!tasks.contains_key("task1"));
    }

    #[test]
    fn test_tasks_config_clone() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "task1".to_string(),
            TaskConfig {
                description: "Test task".to_string(),
                error_message: "Test failed".to_string(),
                task_type: TaskType::RemoteSudo {
                    command: "echo test".to_string(),
                },
            },
        );
        let original = TasksConfig::from(map);

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(cloned.len(), original.len());
        assert_eq!(cloned, original);

        let original_task = original.get("task1").unwrap();
        let cloned_task = cloned.get("task1").unwrap();
        assert_eq!(cloned_task.description, original_task.description);
    }

    #[test]
    fn test_tasks_config_debug() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "debug_task".to_string(),
            TaskConfig {
                description: "Debug task".to_string(),
                error_message: "Debug failed".to_string(),
                task_type: TaskType::RemoteSudo {
                    command: "echo debug".to_string(),
                },
            },
        );
        let tasks = TasksConfig::from(map);

        // When
        let debug_string = format!("{:?}", tasks);

        // Then
        assert!(debug_string.contains("debug_task"));
        assert!(debug_string.contains("Debug task"));
    }

    #[test]
    fn test_tasks_config_partial_eq() {
        // Given
        let mut map1 = HashMap::new();
        map1.insert(
            "task1".to_string(),
            TaskConfig {
                description: "Test task".to_string(),
                error_message: "Test failed".to_string(),
                task_type: TaskType::RemoteSudo {
                    command: "echo test".to_string(),
                },
            },
        );
        let tasks1 = TasksConfig::from(map1);

        let mut map2 = HashMap::new();
        map2.insert(
            "task1".to_string(),
            TaskConfig {
                description: "Test task".to_string(),
                error_message: "Test failed".to_string(),
                task_type: TaskType::RemoteSudo {
                    command: "echo test".to_string(),
                },
            },
        );
        let tasks2 = TasksConfig::from(map2);

        let mut map3 = HashMap::new();
        map3.insert(
            "task2".to_string(),
            TaskConfig {
                description: "Different task".to_string(),
                error_message: "Different failed".to_string(),
                task_type: TaskType::RemoteSudo {
                    command: "echo different".to_string(),
                },
            },
        );
        let tasks3 = TasksConfig::from(map3);

        // When & Then
        assert_eq!(tasks1, tasks2);
        assert_ne!(tasks1, tasks3);
    }
}
