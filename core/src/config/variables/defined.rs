use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct DefinedVariablesConfig(HashMap<String, String>);

impl Deref for DefinedVariablesConfig {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DefinedVariablesConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DefinedVariablesConfig {
    pub fn merge(&self, other: &DefinedVariablesConfig) -> DefinedVariablesConfig {
        let mut merged = self.0.clone();
        for (key, value) in &other.0 {
            merged.insert(key.clone(), value.clone());
        }
        DefinedVariablesConfig(merged)
    }
}
