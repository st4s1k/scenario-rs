use crate::{
    config::RequiredVariablesConfig,
    scenario::errors::RequiredVariablesError,
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub struct RequiredVariables(HashMap<String, String>);

impl Deref for RequiredVariables {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RequiredVariables {
    pub fn new<const N: usize>(variables: [(String, String); N]) -> RequiredVariables {
        RequiredVariables(HashMap::from(variables))
    }

    pub(crate) fn validate(&self, config: &RequiredVariablesConfig) -> Result<(), RequiredVariablesError> {
        let undeclared_but_found =
            self.keys().into_iter()
                .filter(|var| !config.contains(var))
                .map(|var| var.to_string())
                .collect::<Vec<String>>();
        let declared_but_not_found =
            config.iter()
                .filter(|&var| !&self.contains_key(var))
                .map(|var| var.to_string())
                .collect::<Vec<String>>();

        if !undeclared_but_found.is_empty()
            || !declared_but_not_found.is_empty() {
            return Err(RequiredVariablesError::ValidationFailed(undeclared_but_found, declared_but_not_found));
        }

        Ok(())
    }
}
