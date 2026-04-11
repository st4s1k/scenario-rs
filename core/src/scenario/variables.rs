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
    trace::ScenarioEvent,
    utils::{HasPlaceholders, HasText, IsBlank, IsNotEmpty},
};
use regex::Regex;
use std::{collections::HashMap, ops::Deref};
use tracing::debug;

pub mod defined;
pub mod required;
pub mod resolved;

/// Central manager for scenario variables (required + defined) with placeholder resolution.
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
    /// Returns a reference to the required variables.
    pub fn required(&self) -> &RequiredVariables {
        &self.required
    }

    /// Returns a mutable reference to the required variables.
    pub fn required_mut(&mut self) -> &mut RequiredVariables {
        &mut self.required
    }

    /// Returns a reference to the defined variables.
    pub fn defined(&self) -> &DefinedVariables {
        &self.defined
    }

    /// Returns a mutable reference to the defined variables.
    pub fn defined_mut(&mut self) -> &mut DefinedVariables {
        &mut self.defined
    }

    /// Replaces `{env:VAR_NAME}` placeholders with the corresponding environment variable values.
    /// Returns an error immediately if any referenced environment variable is not set.
    fn resolve_env_placeholders(input: &str) -> Result<String, PlaceholderResolutionError> {
        let env_regex =
            Regex::new(r"\{env:([^}]+)\}").expect("env_regex should be a valid regex");

        let mut output = input.to_string();

        for captures in env_regex.captures_iter(input) {
            let full_match = captures.get(0).unwrap().as_str();
            let var_name = captures.get(1).unwrap().as_str();

            let value = std::env::var(var_name).map_err(|_| {
                PlaceholderResolutionError::CannotResolveEnvVariable(var_name.to_string())
            })?;

            output = output.replace(full_match, &value);
        }

        Ok(output)
    }

    /// Replaces `{variable_name}` placeholders in the input string, supporting nested resolution.
    /// Also resolves `{env:VAR_NAME}` placeholders from environment variables.
    pub fn resolve_placeholders(&self, input: &str) -> Result<String, PlaceholderResolutionError> {
        if !input.has_placeholders() {
            return Ok(input.to_string());
        }

        // Pre-pass: resolve {env:VAR_NAME} placeholders from environment
        let mut output = Self::resolve_env_placeholders(input)?;

        if !output.has_placeholders() {
            return Ok(output);
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

    /// Resolves all placeholders across all variables, returning a fully resolved snapshot.
    pub fn resolved(&self) -> Result<ResolvedVariables, PlaceholderResolutionError> {
        let mut all_variables = HashMap::new();

        all_variables.extend(self.defined.deref().clone());
        all_variables.extend(self.required.value_map());

        all_variables
            .iter()
            .filter(|(_, value)| value.is_blank())
            .for_each(|(key, _)| {
                debug!(
                    scenario.event = ScenarioEvent::Error.as_str(),
                    scenario.error = format!("Variable '{}' has a blank value", key)
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

#[cfg(test)]
mod tests {
    use crate::{
        config::variables::{
            defined::DefinedVariablesConfig,
            required::{RequiredVariableConfig, RequiredVariablesConfig, VariableTypeConfig},
            VariablesConfig,
        },
        scenario::{
            errors::PlaceholderResolutionError,
            variables::{
                defined::DefinedVariables,
                required::{RequiredVariable, RequiredVariables},
                Variables,
            },
        },
    };
    use std::collections::HashMap;
    use std::ops::Deref;

    #[test]
    fn test_variables_default() {
        // Given & When
        let variables = Variables::default();

        // Then
        assert!(variables.required().is_empty());
        assert!(variables.defined().is_empty());
    }

    #[test]
    fn test_variables_from_config() {
        // Given
        let mut required_map = HashMap::new();
        required_map.insert(
            "username".to_string(),
            RequiredVariableConfig {
                label: Some("Username".to_string()),
                var_type: VariableTypeConfig::String,
                read_only: false,
            },
        );
        let required_config = RequiredVariablesConfig::from(required_map);

        let mut defined_map = HashMap::new();
        defined_map.insert("hostname".to_string(), "example.com".to_string());
        let defined_config = DefinedVariablesConfig::from(defined_map);

        let config = VariablesConfig {
            required: required_config,
            defined: defined_config,
        };

        // When
        let variables = Variables::from(&config);

        // Then
        assert_eq!(variables.required().len(), 1);
        assert!(variables.required().contains_key("username"));

        assert_eq!(variables.defined().len(), 1);
        assert_eq!(
            variables.defined().get("hostname"),
            Some(&"example.com".to_string())
        );
    }

    #[test]
    fn test_variables_resolve_no_placeholders() {
        // Given
        let variables = Variables::default();
        let input = "Hello, world!";

        // When
        let result = variables.resolve_placeholders(input);

        // Then
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");
    }

    #[test]
    fn test_variables_resolve_simple_placeholders() {
        // Given
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("name".to_string(), "Alice".to_string());
        variables.required_mut().insert(
            "greeting".to_string(),
            RequiredVariable::default().with_value("Hello".to_string()),
        );

        // When
        let result = variables.resolve_placeholders("{greeting}, {name}!");

        // Then
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, Alice!");
    }

    #[test]
    fn test_variables_resolve_nested_placeholders() {
        // Given
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("app_name".to_string(), "my-service".to_string());
        variables
            .defined_mut()
            .insert("env".to_string(), "production".to_string());
        variables.defined_mut().insert(
            "log_dir".to_string(),
            "/var/log/{app_name}/{env}".to_string(),
        );

        // When
        let result = variables.resolve_placeholders("{log_dir}/app.log");

        // Then
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/var/log/my-service/production/app.log");
    }

    #[test]
    fn test_variables_resolve_placeholder_error() {
        // Given
        let variables = Variables::default();

        // When
        let result = variables.resolve_placeholders("Hello, {missing_var}!");

        // Then
        assert!(result.is_err());
        if let Err(PlaceholderResolutionError::CannotResolvePlaceholders(input)) = result {
            assert_eq!(input, "Hello, {missing_var}!");
        } else {
            panic!("Expected CannotResolvePlaceholders error");
        }
    }

    #[test]
    fn test_variables_resolve_circular_reference_error() {
        // Given
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("var1".to_string(), "{var2}".to_string());
        variables
            .defined_mut()
            .insert("var2".to_string(), "{var1}".to_string());

        // When
        let result = variables.resolve_placeholders("{var1}");

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_variables_resolved_success() {
        // Given
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .extend(create_test_defined_variables().deref().clone());
        variables
            .required_mut()
            .extend(create_test_required_variables().deref().clone());

        // When
        let result = variables.resolved();

        // Then
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(
            resolved.get("url"),
            Some(&"https://example.com:8080".to_string())
        );
        assert_eq!(resolved.get("username"), Some(&"admin".to_string()));
    }

    #[test]
    fn test_variables_resolved_error() {
        // Given
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("url".to_string(), "https://{hostname}:{port}".to_string());

        // When
        let result = variables.resolved();

        // Then
        assert!(result.is_err());
        if let Err(PlaceholderResolutionError::CannotResolveVariablesPlaceholders(unresolved)) =
            result
        {
            assert!(unresolved.contains(&"url".to_string()));
        } else {
            panic!("Expected CannotResolveVariablesPlaceholders error");
        }
    }

    #[test]
    fn test_variables_getters() {
        // Given
        let mut variables = Variables::default();
        let required = create_test_required_variables();
        let defined = create_test_defined_variables();
        variables.required_mut().extend(required.deref().clone());
        variables.defined_mut().extend(defined.deref().clone());

        // When & Then
        assert_eq!(variables.required().len(), required.len());
        assert_eq!(variables.defined().len(), defined.len());

        assert_eq!(
            variables.required().get("username").unwrap().value(),
            "admin"
        );
        assert_eq!(variables.defined().get("hostname").unwrap(), "example.com");
    }

    #[test]
    fn test_variables_clone() {
        // Given
        let mut original = Variables::default();
        original
            .defined_mut()
            .insert("key".to_string(), "value".to_string());
        original.required_mut().insert(
            "req".to_string(),
            RequiredVariable::default().with_value("req-value".to_string()),
        );

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(cloned.required().len(), original.required().len());
        assert_eq!(cloned.defined().len(), original.defined().len());
        assert_eq!(cloned.required().get("req").unwrap().value(), "req-value");
        assert_eq!(cloned.defined().get("key").unwrap(), "value");
    }

    #[test]
    fn test_variables_debug() {
        // Given
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("debug_key".to_string(), "debug_value".to_string());

        // When
        let debug_string = format!("{:?}", variables);

        // Then
        assert!(debug_string.contains("debug_key"));
        assert!(debug_string.contains("debug_value"));
    }

    fn create_test_required_variables() -> RequiredVariables {
        let mut required = RequiredVariables::default();
        required.insert(
            "username".to_string(),
            RequiredVariable::default()
                .with_label("Username".to_string())
                .with_value("admin".to_string()),
        );
        required.insert(
            "password".to_string(),
            RequiredVariable::default()
                .with_label("Password".to_string())
                .with_value("secret".to_string()),
        );
        required
    }

    fn create_test_defined_variables() -> DefinedVariables {
        let mut defined_vars = HashMap::new();
        defined_vars.insert("hostname".to_string(), "example.com".to_string());
        defined_vars.insert("port".to_string(), "8080".to_string());
        defined_vars.insert("url".to_string(), "https://{hostname}:{port}".to_string());
        DefinedVariables::from(defined_vars)
    }

    #[test]
    fn test_variables_resolve_filters_empty_values() {
        // Given
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("name".to_string(), "Alice".to_string());
        variables.required_mut().insert(
            "empty_var".to_string(),
            RequiredVariable::default().with_value("".to_string()),
        );

        // When
        let result = variables.resolve_placeholders("{name}");

        // Then
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Alice");
    }

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::TRACE)
            .try_init();
    }

    #[test]
    fn test_variables_resolved_with_blank_values_logs_warning() {
        init_tracing();
        // Given
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("hostname".to_string(), "example.com".to_string());
        variables
            .defined_mut()
            .insert("empty_key".to_string(), "".to_string());

        // When
        let result = variables.resolved();

        // Then
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.get("hostname"), Some(&"example.com".to_string()));
    }

    #[test]
    fn test_resolve_env_variable_success() {
        // Given
        std::env::set_var("SCENARIO_RS_TEST_VAR", "env_value");
        let variables = Variables::default();

        // When
        let result = variables.resolve_placeholders("prefix-{env:SCENARIO_RS_TEST_VAR}-suffix");

        // Then
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "prefix-env_value-suffix");

        std::env::remove_var("SCENARIO_RS_TEST_VAR");
    }

    #[test]
    fn test_resolve_env_variable_missing() {
        // Given
        std::env::remove_var("SCENARIO_RS_MISSING_VAR");
        let variables = Variables::default();

        // When
        let result = variables.resolve_placeholders("value-{env:SCENARIO_RS_MISSING_VAR}");

        // Then
        assert!(result.is_err());
        if let Err(PlaceholderResolutionError::CannotResolveEnvVariable(name)) = result {
            assert_eq!(name, "SCENARIO_RS_MISSING_VAR");
        } else {
            panic!("Expected CannotResolveEnvVariable error");
        }
    }

    #[test]
    fn test_resolve_env_variable_combined_with_regular() {
        // Given
        std::env::set_var("SCENARIO_RS_TEST_USER", "deploy_user");
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("service_name".to_string(), "my-app".to_string());

        // When
        let result =
            variables.resolve_placeholders("/backup/{service_name}/{env:SCENARIO_RS_TEST_USER}");

        // Then
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/backup/my-app/deploy_user");

        std::env::remove_var("SCENARIO_RS_TEST_USER");
    }

    #[test]
    fn test_resolved_with_env_variables_in_defined() {
        // Given
        std::env::set_var("SCENARIO_RS_TEST_HOST", "prod.example.com");
        let mut variables = Variables::default();
        variables.defined_mut().insert(
            "url".to_string(),
            "https://{env:SCENARIO_RS_TEST_HOST}/api".to_string(),
        );

        // When
        let result = variables.resolved();

        // Then
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(
            resolved.get("url"),
            Some(&"https://prod.example.com/api".to_string())
        );

        std::env::remove_var("SCENARIO_RS_TEST_HOST");
    }
}
