use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct RequiredVariablesConfig(pub HashMap<String, RequiredVariableConfig>);

impl Deref for RequiredVariablesConfig {
    type Target = HashMap<String, RequiredVariableConfig>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariablesConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RequiredVariablesConfig {
    pub fn merge(&self, other: &RequiredVariablesConfig) -> RequiredVariablesConfig {
        let mut merged = self.0.clone();
        for (key, value) in &other.0 {
            merged.insert(key.clone(), value.clone());
        }
        RequiredVariablesConfig(merged)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct RequiredVariableConfig {
    #[serde(flatten)]
    pub var_type: VariableTypeConfig,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum VariableTypeConfig {
    String,
    Path,
    Timestamp { format: String },
}
