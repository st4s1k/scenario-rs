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
