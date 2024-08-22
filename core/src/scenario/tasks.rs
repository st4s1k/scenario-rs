use crate::scenario::errors::TasksError;
use crate::scenario::variables::Variables;
use crate::{
    config::TasksConfig,
    scenario::task::Task,
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
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

impl Tasks {
    pub(crate) fn resolve_placeholders(&mut self, variables: &Variables) -> Result<(), TasksError> {
        let unresolved_tasks = self.deref_mut().iter_mut()
            .map(|(id, task)| (id, task.resolve_placeholders(variables)))
            .filter(|(_, result)| result.is_err())
            .map(|(id, _)| id.to_string())
            .collect::<Vec<String>>();

        if !unresolved_tasks.is_empty() {
            return Err(TasksError::CannotResolvePlaceholdersInTasks(unresolved_tasks));
        }

        Ok(())
    }
}
