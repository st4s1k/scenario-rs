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
#[derive(Deserialize, Clone, Debug, Default)]
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
