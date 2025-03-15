pub mod required;

use crate::{
    config::{SpecialVariablesConfig, VariablesConfig},
    scenario::{
        errors::PlaceholderResolutionError, utils::HasPlaceholders,
        variables::required::RequiredVariables,
    },
};
use chrono::Local;
use std::{collections::HashMap, ops::Deref, path::PathBuf, str::FromStr};

#[derive(Clone, Debug)]
pub struct Variables {
    required: RequiredVariables,
    defined: HashMap<String, String>,
}

impl Default for Variables {
    fn default() -> Self {
        Variables {
            required: RequiredVariables::default(),
            defined: HashMap::new(),
        }
    }
}

impl From<&VariablesConfig> for Variables {
    fn from(config: &VariablesConfig) -> Self {
        let mut variables_map = HashMap::<String, String>::new();
        variables_map.extend(config.defined.deref().clone());

        for (key, value) in &variables_map.clone() {
            if key.starts_with("path:") {
                PathBuf::from_str(value.as_str())
                    .ok()
                    .and_then(|path| path.file_name().map(|file_name| file_name.to_owned()))
                    .and_then(|file_name| file_name.to_str().map(|s| s.to_string()))
                    .map(|file_name| {
                        let basename_key = key.replace("path:", "basename:");
                        variables_map.insert(basename_key, file_name.to_string());
                    });
            }
        }
        let mut variables = Variables {
            required: RequiredVariables::from(&config.required),
            defined: variables_map,
        };
        variables._resolve_special_variables(&config.special);
        variables
    }
}

impl Variables {
    pub fn resolved(&self) -> Result<HashMap<String, String>, PlaceholderResolutionError> {
        Ok(self._resolve_placeholders()?)
    }

    pub fn required(&self) -> &RequiredVariables {
        &self.required
    }

    pub fn required_mut(&mut self) -> &mut RequiredVariables {
        &mut self.required
    }

    pub(crate) fn resolve_placeholders(
        &self,
        input: &str,
    ) -> Result<String, PlaceholderResolutionError> {
        if !input.has_placeholders() {
            return Ok(input.to_string());
        }

        let mut variables = self
            .defined
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect::<HashMap<&str, &str>>();

        self.required.iter().for_each(|(name, required_variable)| {
            variables.insert(name.as_str(), required_variable.value.as_str());
        });

        let mut output = input.to_string();

        loop {
            let previous = output.clone();

            for (key, value) in &variables {
                let placeholder = format!("{{{}}}", key);
                output = output.replace(&placeholder, value);
            }

            if !output.has_placeholders() {
                return Ok(output);
            }

            if output == previous {
                return Err(PlaceholderResolutionError::CannotResolvePlaceholders(
                    output,
                ));
            }
        }
    }

    fn _resolve_special_variables(&mut self, config: &SpecialVariablesConfig) {
        if let Some(timestamp_format) = &config.get("timestamp") {
            let timestamp: String = Local::now().format(timestamp_format).to_string();
            self.defined.insert("timestamp".to_string(), timestamp);
        }
    }

    fn _resolve_placeholders(&self) -> Result<HashMap<String, String>, PlaceholderResolutionError> {
        let mut resolved_variables = self.defined.clone();
        self.required.iter().for_each(|(name, required_variable)| {
            resolved_variables.insert(name.clone(), required_variable.value.clone());
        });

        loop {
            let mut changes = false;
            for key in &resolved_variables.keys().cloned().collect::<Vec<String>>() {
                let value = &resolved_variables[key];
                let new_value = self.resolve_placeholders(value)?;
                if new_value != resolved_variables[key] {
                    resolved_variables.insert(key.to_string(), new_value);
                    changes = true;
                }
            }
            if !changes {
                break;
            }
        }

        let unresolved_keys = resolved_variables
            .iter()
            .filter(|(_, value)| value.has_placeholders())
            .map(|(key, _)| key.to_owned())
            .collect::<Vec<String>>();

        if !unresolved_keys.is_empty() {
            return Err(
                PlaceholderResolutionError::CannotResolveVariablesPlaceholders(unresolved_keys),
            );
        }

        Ok(resolved_variables)
    }
}
