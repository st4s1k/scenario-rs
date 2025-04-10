//! Variable management for scenarios.
//!
//! This module provides functionality for managing and resolving variables in scenarios,
//! including variable placeholder substitution and handling different variable types.

use crate::{
    config::variables::VariablesConfig,
    scenario::{
        errors::PlaceholderResolutionError,
        variables::{
            defined::DefinedVariables, required::RequiredVariables, resolved::ResolvedVariables,
        },
    },
    utils::{HasPlaceholders, HasText, IsBlank, IsNotEmpty},
};
use std::{collections::HashMap, ops::Deref};
use tracing::debug;

pub mod defined;
pub mod required;
pub mod resolved;

/// Central manager for scenario variables.
///
/// This struct coordinates both required and defined variables, and provides
/// functionality for resolving variable placeholders in strings. Required variables
/// are those that must be provided at runtime, while defined variables are
/// predefined in the scenario configuration.
///
/// # Example
///
/// ```no_run
/// # use scenario_rs::config::variables::VariablesConfig;
/// # use std::collections::HashMap;
/// # use scenario_rs::scenario::variables::Variables;
/// #
/// // Create a Variables instance from a configuration
/// let mut vars = Variables::default();
///
/// // Resolve placeholders in a string
/// let resolved = vars.resolve_placeholders("Hello, {username}!")
///     .expect("Failed to resolve placeholders");
/// ```
#[derive(Clone, Debug)]
pub struct Variables {
    /// Variables that must be provided at runtime
    required: RequiredVariables,
    /// Variables predefined in the scenario configuration
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
    /// Returns a reference to the required variables.
    pub fn required(&self) -> &RequiredVariables {
        &self.required
    }

    /// Returns a mutable reference to the required variables.
    pub fn required_mut(&mut self) -> &mut RequiredVariables {
        &mut self.required
    }

    /// Resolves all variable placeholders in a string.
    ///
    /// This method replaces occurrences of `{variable_name}` in the input string
    /// with the corresponding variable value. It supports nested variables, where
    /// a variable's value may itself contain placeholders that need resolving.
    ///
    /// # Arguments
    ///
    /// * `input` - The string containing placeholders to resolve
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The input string with all placeholders replaced
    /// * `Err(PlaceholderResolutionError)` - If placeholders can't be resolved
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scenario_rs::scenario::variables::Variables;
    /// #
    /// let vars = Variables::default();
    /// let result = vars.resolve_placeholders("Hello, {name}!");
    /// ```
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

        variables = variables
            .iter()
            .filter(|(_, value)| value.has_text())
            .map(|(key, value)| (*key, *value))
            .collect::<HashMap<&str, &str>>();

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
                    input.to_string(),
                ));
            }
        }
    }

    /// Creates a fully resolved view of all variables.
    ///
    /// This method attempts to resolve all placeholders in all variables until
    /// no more resolutions are possible. It returns a ResolvedVariables instance
    /// containing the fully resolved values.
    ///
    /// # Returns
    ///
    /// * `Ok(ResolvedVariables)` - All variables successfully resolved
    /// * `Err(PlaceholderResolutionError)` - If some placeholders can't be resolved
    pub fn resolved(&self) -> Result<ResolvedVariables, PlaceholderResolutionError> {
        let mut all_variables = HashMap::new();

        all_variables.extend(self.defined.deref().clone());
        all_variables.extend(self.required.value_map());

        all_variables
            .iter()
            .filter(|(_, value)| value.is_blank())
            .for_each(|(key, _)| {
                debug!(
                    event = "error",
                    error = format!("Variable '{}' has a blank value", key)
                );
            });

        loop {
            let mut resolved_variables = HashMap::new();

            for (variable_name, value) in all_variables.iter() {
                if let Ok(new_value) = self.resolve_placeholders(value) {
                    if new_value != *value {
                        resolved_variables.insert(variable_name.clone(), new_value);
                    }
                };
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

        if unresolved_variable_names.is_not_empty() {
            return Err(
                PlaceholderResolutionError::CannotResolveVariablesPlaceholders(
                    unresolved_variable_names,
                ),
            );
        }

        Ok(ResolvedVariables(all_variables))
    }
}
