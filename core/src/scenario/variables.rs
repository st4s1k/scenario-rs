use crate::{
    config::{SpecialVariablesConfig, VariablesConfig},
    scenario::{
        errors::{PlaceholderResolutionError, VariablesError},
        utils::HasPlaceholders,
        variables::required::RequiredVariables,
    },
};
use chrono::Local;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub mod required;

pub struct Variables(HashMap<String, String>);

impl Deref for Variables {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Variables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<(&RequiredVariables, &VariablesConfig)> for Variables {
    type Error = VariablesError;

    fn try_from((required_variables, config): (&RequiredVariables, &VariablesConfig)) -> Result<Self, Self::Error> {
        required_variables
            .validate(&config.required)
            .map_err(VariablesError::RequiredVariablesValidationFailed)?;

        let mut variables_map = HashMap::<String, String>::new();
        variables_map.extend(required_variables.deref().clone());
        variables_map.extend(config.defined.clone());

        let mut variables = Variables(variables_map);

        variables._resolve_special_variables(&config.special);
        variables._resolve_placeholders()
            .map_err(VariablesError::CannotResolveVariablesPlaceholders)?;

        Ok(variables)
    }
}

impl Variables {
    pub(crate) fn resolve_placeholders(&self, input: &str) -> Result<String, PlaceholderResolutionError> {
        let mut output = input.to_string();
        for (key, value) in self.deref() {
            output = output.replace(&format!("{{{key}}}"), value);
        }
        if output.has_placeholders() {
            return Err(PlaceholderResolutionError::UnresolvedValue(output));
        }
        Ok(output)
    }

    fn _resolve_special_variables(&mut self, config: &SpecialVariablesConfig) {
        if let Some(timestamp_format) = &config.get("timestamp") {
            let timestamp: String = Local::now().format(timestamp_format).to_string();
            self.insert("timestamp".to_string(), timestamp);
        }
    }

    fn _resolve_placeholders(&mut self) -> Result<(), PlaceholderResolutionError> {
        let mut iterations = 0;
        let max_iterations = 10;
        while iterations < max_iterations {
            let mut changes = false;
            for key in self.to_owned().keys().cloned() {
                let variables = &self;
                let value = &variables[&key];
                let new_value = self.resolve_placeholders(value)?;
                if new_value != variables[&key] {
                    self.insert(key, new_value);
                    changes = true;
                }
            }
            if !changes {
                break;
            }
            iterations += 1;
        }

        let unresolved_keys = self.iter()
            .filter(|(_, value)| value.has_placeholders())
            .map(|(key, _)| key.to_owned())
            .collect::<Vec<String>>();

        if !unresolved_keys.is_empty() {
            return Err(PlaceholderResolutionError::UnresolvedValues(unresolved_keys));
        }

        Ok(())
    }
}
