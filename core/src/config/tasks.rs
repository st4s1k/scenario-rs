use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::Deserialize;

use super::task::TaskConfig;

#[derive(Deserialize, Clone, Debug)]
pub struct TasksConfig(HashMap<String, TaskConfig>);

impl Deref for TasksConfig {
    type Target = HashMap<String, TaskConfig>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TasksConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
