pub mod defined;
pub mod required;
pub mod resolved;

use defined::DefinedVariables;
use resolved::ResolvedVariables;

use crate::{
    config::variables::VariablesConfig,
    scenario::{
        errors::PlaceholderResolutionError, utils::HasPlaceholders,
        variables::required::RequiredVariables,
    },
};
use std::{collections::HashMap, ops::Deref};

#[derive(Clone, Debug)]
pub struct Variables {
    required: RequiredVariables,
    defined: DefinedVariables,
}

impl Default for Variables {
    fn default() -> Self {
        Variables {
            required: RequiredVariables::default(),
            defined: DefinedVariables::default(),
        }
    }
}

impl From<&VariablesConfig> for Variables {
    fn from(config: &VariablesConfig) -> Self {
        Variables {
            required: RequiredVariables::from(&config.required),
            defined: DefinedVariables::from(&config.defined),
        }
    }
}

impl Variables {
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

    pub fn resolved(&self) -> Result<ResolvedVariables, PlaceholderResolutionError> {
        let mut all_variables = HashMap::new();

        all_variables.extend(self.defined.deref().clone());
        all_variables.extend(self.required.value_map());

        loop {
            let mut resolved_variables = HashMap::new();

            for (variable_name, value) in all_variables.iter() {
                let new_value = self.resolve_placeholders(value)?;
                if new_value != *value {
                    resolved_variables.insert(variable_name.clone(), new_value);
                }
            }

            if resolved_variables.is_empty() {
                break;
            }

            all_variables.extend(resolved_variables);
        }

        let unresolved_variable_names: Vec<String> = all_variables
            .iter()
            .filter(|(_, value)| value.has_placeholders())
            .map(|(name, _)| name.clone())
            .collect();

        if !unresolved_variable_names.is_empty() {
            return Err(
                PlaceholderResolutionError::CannotResolveVariablesPlaceholders(unresolved_variable_names),
            );
        }

        Ok(ResolvedVariables(all_variables))
    }
}
