//! Task registry for scenarios.
//!
//! This module provides a registry of tasks that can be referenced and executed
//! as part of scenario steps. It maps task names to their implementations.

use crate::{config::tasks::TasksConfig, scenario::task::Task};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// A collection of available tasks in a scenario.
///
/// This struct maps task identifiers to their implementations, serving as a registry
/// that steps can reference by name. Tasks are typically defined in configuration
/// files and then instantiated for execution.
///
/// # Examples
///
/// Creating a task registry:
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_core::{
///     config::task::{TaskConfig, TaskType},
///     scenario::{task::Task, tasks::Tasks}
/// };
///
/// // Create task configurations
/// let mut task_map = HashMap::new();
///
/// // Add a remote command task
/// let start_service_config = TaskConfig {
///     description: "Start the application service".to_string(),
///     error_message: "Failed to start service".to_string(),
///     task_type: TaskType::RemoteSudo {
///         command: "systemctl start myapp".to_string(),
///     },
/// };
/// let start_service_task = Task::from(&start_service_config);
/// task_map.insert("start_service".to_string(), start_service_task);
///
/// // Add a file transfer task
/// let deploy_config_config = TaskConfig {
///     description: "Deploy configuration file".to_string(),
///     error_message: "Failed to deploy config".to_string(),
///     task_type: TaskType::SftpCopy {
///         source_path: "./config.yaml".to_string(),
///         destination_path: "/etc/myapp/config.yaml".to_string(),
///     },
/// };
/// let deploy_config_task = Task::from(&deploy_config_config);
/// task_map.insert("deploy_config".to_string(), deploy_config_task);
///
/// // Create the Tasks registry
/// let tasks = Tasks(task_map);
///
/// // Access tasks by name
/// assert!(tasks.contains_key("start_service"));
/// assert!(tasks.contains_key("deploy_config"));
/// assert_eq!(tasks.len(), 2);
///
/// // Retrieve task details
/// let start_task = tasks.get("start_service").unwrap();
/// assert_eq!(start_task.description(), "Start the application service");
/// ```
#[derive(Clone, Debug)]
pub struct Tasks(pub HashMap<String, Task>);

impl Deref for Tasks {
    type Target = HashMap<String, Task>;
    
    /// Dereferences to the underlying HashMap of tasks.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tasks {
    /// Provides mutable access to the underlying HashMap of tasks.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&TasksConfig> for Tasks {
    /// Creates a Tasks collection from a TasksConfig.
    ///
    /// This converts a configuration structure into a runtime tasks registry,
    /// instantiating each task from its configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The tasks configuration from which to create tasks
    ///
    /// # Returns
    ///
    /// A new Tasks instance containing all the tasks defined in the configuration
    fn from(config: &TasksConfig) -> Self {
        let mut tasks = HashMap::<String, Task>::new();

        for (id, task_config) in config.deref() {
            let task = Task::from(task_config);
            tasks.insert(id.clone(), task);
        }

        Tasks(tasks)
    }
}
