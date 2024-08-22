use crate::{
    config::TasksConfig,
    scenario::{
        errors::TasksError,
        task::Task,
        variables::Variables,
    },
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

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

impl TryFrom<(&TasksConfig, &Variables)> for Tasks {
    type Error = TasksError;
    fn try_from((config, variables): (&TasksConfig, &Variables)) -> Result<Self, Self::Error> {
        let mut tasks = HashMap::<String, Task>::new();

        for (id, task_config) in config.deref() {
            let task = Task::try_from((task_config, variables))
                .map_err(TasksError::CannotCreateTaskFromConfig)?;
            tasks.insert(id.clone(), task);
        }

        Ok(Tasks(tasks))
    }
} 
