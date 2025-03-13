use crate::{config::TasksConfig, scenario::task::Task};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug)]
pub struct Tasks(pub HashMap<String, Task>);

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
