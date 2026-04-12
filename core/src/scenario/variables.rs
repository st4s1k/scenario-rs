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
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Local;
use regex::Regex;
use std::{collections::HashMap, env, ops::Deref, path::Path};
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

    /// Returns built-in variables that can be treated like stable plain variables.
    fn builtin_variables() -> HashMap<&'static str, String> {
        let mut builtins = HashMap::new();
        builtins.insert(
            "hostname",
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_default(),
        );
        builtins.insert("os", env::consts::OS.to_string());
        builtins
    }

    /// Resolves zero-input built-ins like `{now}` per occurrence.
    ///
    /// User-defined variables keep precedence: if `variables` already contains one of these names,
    /// the placeholder is left unchanged for the regular variable-substitution loop.
    fn resolve_zero_input_placeholders(input: &str, variables: &HashMap<&str, &str>) -> String {
        let zero_input_regex = Regex::new(r"\{(hostname|os|now)\}")
            .expect("zero_input_regex should be a valid regex");

        zero_input_regex
            .replace_all(input, |captures: &regex::Captures| {
                let placeholder_name = captures.get(1).unwrap().as_str();

                if variables.contains_key(placeholder_name) {
                    captures.get(0).unwrap().as_str().to_string()
                } else {
                    match placeholder_name {
                        "hostname" => hostname::get()
                            .ok()
                            .and_then(|h| h.into_string().ok())
                            .unwrap_or_default(),
                        "os" => env::consts::OS.to_string(),
                        "now" => Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
                        #[cfg(not(tarpaulin_include))]
                        _ => captures.get(0).unwrap().as_str().to_string(),
                    }
                }
            })
            .into_owned()
    }

    /// Replaces every `{uuid}` occurrence with a freshly generated UUID v4,
    /// so each placeholder gets its own unique value.
    fn resolve_uuid_placeholders(input: &str) -> String {
        let uuid_regex = Regex::new(r"\{uuid\}")
            .expect("uuid_regex should be a valid regex");

        uuid_regex.replace_all(input, |_: &regex::Captures| {
            uuid::Uuid::new_v4().to_string()
        }).into_owned()
    }

    /// Translates a human-readable datetime format to a chrono strftime format.
    /// Tokens are replaced longest-first to avoid partial matches.
    ///
    /// Supported tokens: `YYYY`, `YY`, `MM`, `DD`, `HH`, `hh`, `mm`, `ss`, `SSS`, `Z`
    fn translate_datetime_format(format: &str) -> String {
        let tokens = [
            ("YYYY", "%Y"),
            ("YY", "%y"),
            ("MM", "%m"),
            ("DD", "%d"),
            ("HH", "%H"),
            ("hh", "%I"),
            ("mm", "%M"),
            ("ss", "%S"),
            ("SSS", "%3f"),
            ("Z", "%:z"),
        ];

        let mut result = format.to_string();
        // Replace longest tokens first to avoid partial matches
        let mut sorted_tokens = tokens.to_vec();
        sorted_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (token, chrono_fmt) in &sorted_tokens {
            result = result.replace(token, chrono_fmt);
        }

        result
    }

    /// Resolves `{now:format}` placeholders with a custom datetime format.
    fn resolve_now_placeholders(input: &str) -> String {
        let now_regex = Regex::new(r"\{now:([^}]+)\}")
            .expect("now_regex should be a valid regex");

        now_regex
            .replace_all(input, |captures: &regex::Captures| {
                let format_arg = captures.get(1).unwrap().as_str();
                let now = Local::now();

                match format_arg {
                    "epoch" => now.timestamp().to_string(),
                    "epoch_ms" => now.timestamp_millis().to_string(),
                    _ => {
                        let chrono_fmt = Self::translate_datetime_format(format_arg);
                        now.format(&chrono_fmt).to_string()
                    }
                }
            })
            .into_owned()
    }

    /// Resolves `{modifier:var}` patterns by looking up `var` in the provided variables map
    /// and applying the modifier function. Supports path modifiers (`basename`, `stem`, `dir`,
    /// `ext`, `abspath`) and string modifiers (`uppercase`, `lowercase`, `base64`, `trim`).
    fn resolve_modifier_placeholders(
        input: &str,
        variables: &HashMap<&str, &str>,
    ) -> String {
        let modifier_regex = Regex::new(r"\{(basename|stem|dir|ext|abspath|uppercase|lowercase|base64|trim):([^}]+)\}")
            .expect("modifier_regex should be a valid regex");

        modifier_regex
            .replace_all(input, |captures: &regex::Captures| {
                let modifier = captures.get(1).unwrap().as_str();
                let var_name = captures.get(2).unwrap().as_str();

                variables.get(var_name).map_or_else(
                    || captures.get(0).unwrap().as_str().to_string(),
                    |&var_value| match modifier {
                        "basename" => Path::new(var_value)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or(var_value)
                            .to_string(),
                        "stem" => Path::new(var_value)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(var_value)
                            .to_string(),
                        "dir" => Path::new(var_value)
                            .parent()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string(),
                        "ext" => Path::new(var_value)
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string(),
                        "abspath" => std::fs::canonicalize(var_value)
                            .ok()
                            .and_then(|p| p.to_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| {
                                env::current_dir()
                                    .ok()
                                    .map(|cwd| cwd.join(var_value).to_string_lossy().to_string())
                                    .unwrap_or_else(|| var_value.to_string())
                            }),
                        "uppercase" => var_value.to_uppercase(),
                        "lowercase" => var_value.to_lowercase(),
                        "base64" => BASE64.encode(var_value),
                        "trim" => var_value.trim().to_string(),
                        #[cfg(not(tarpaulin_include))]
                        _ => captures.get(0).unwrap().as_str().to_string(),
                    },
                )
            })
            .into_owned()
    }

    /// Replaces `{variable_name}` placeholders in the input string, supporting nested resolution.
    /// Also resolves `{env:VAR_NAME}`, built-in zero-input, and `{modifier:var}` placeholders.
    pub fn resolve_placeholders(&self, input: &str) -> Result<String, PlaceholderResolutionError> {
        if !input.has_placeholders() {
            return Ok(input.to_string());
        }

        // Pre-pass: resolve {env:VAR_NAME} placeholders from environment
        let mut output = Self::resolve_env_placeholders(input)?;

        if !output.has_placeholders() {
            return Ok(output);
        }

        // Build the variables map: user-defined + required
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

        // Inject stable built-in zero-input variables (user variables take precedence)
        let builtins = Self::builtin_variables();
        let builtin_refs: HashMap<&str, &str> = builtins
            .iter()
            .filter(|(key, _)| !variables.contains_key(*key))
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        let mut all_variables = variables.clone();
        all_variables.extend(builtin_refs);

        // Main substitution loop
        loop {
            let previous = output.clone();

            for (key, value) in &all_variables {
                let placeholder = format!("{{{}}}", key);
                output = output.replace(&placeholder, value);
            }

            // Resolve {env:VAR} that appeared after variable expansion
            if output.has_placeholders() {
                output = Self::resolve_env_placeholders(&output)?;
            }

            // Resolve {now:format} that appeared after variable expansion
            if output.has_placeholders() {
                output = Self::resolve_now_placeholders(&output);
            }

            // Resolve zero-input built-ins like {now} after variable expansion.
            if output.has_placeholders() {
                output = Self::resolve_zero_input_placeholders(&output, &variables);
            }

            if !output.has_placeholders() {
                return Ok(output);
            }

            if output == previous {
                break;
            }
        }

        // Post-pass: resolve {modifier:var} placeholders
        if output.has_placeholders() {
            output = Self::resolve_modifier_placeholders(&output, &all_variables);
        }

        // Resolve {uuid} — each occurrence gets a unique value
        if output.has_placeholders() {
            output = Self::resolve_uuid_placeholders(&output);
        }

        if output.has_placeholders() {
            return Err(PlaceholderResolutionError::CannotResolvePlaceholders(
                input.to_string(),
            ));
        }

        Ok(output)
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
            required::{RequiredVariableConfig, RequiredVariablesConfig},
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
                ..Default::default()
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

    #[test]
    fn test_env_and_now_resolved_after_variable_expansion() {
        // Given — {env:...} and {now:...} appear only after expanding defined variables
        std::env::set_var("SCENARIO_RS_TEST_DEPLOY_USER", "deployer");
        let mut variables = Variables::default();
        variables.defined_mut().insert(
            "service_name".to_string(),
            "my-app".to_string(),
        );
        variables.required_mut().insert(
            "timestamp".to_string(),
            RequiredVariable::default()
                .with_value("{now:YYYY-MM-DD}".to_string())
                .with_read_only(true),
        );
        variables.defined_mut().insert(
            "backup_path".to_string(),
            "/backup/{service_name}/{service_name}-{timestamp}.{env:SCENARIO_RS_TEST_DEPLOY_USER}.jar".to_string(),
        );

        // When
        let result = variables.resolve_placeholders("cp -a /deploy/app.jar {backup_path}");

        // Then
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(resolved.starts_with("cp -a /deploy/app.jar /backup/my-app/my-app-"));
        assert!(resolved.ends_with(".deployer.jar"));
        // Verify date portion was resolved (not raw {now:...})
        assert!(!resolved.contains("{now:"));
        assert!(!resolved.contains("{env:"));

        std::env::remove_var("SCENARIO_RS_TEST_DEPLOY_USER");
    }

    // --- Path modifier tests ---

    #[test]
    fn test_modifier_basename() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("jar_path".to_string(), "/usr/src/app.jar".to_string());

        let result = variables.resolve_placeholders("{basename:jar_path}");
        assert_eq!(result.unwrap(), "app.jar");
    }

    #[test]
    fn test_modifier_stem() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("jar_path".to_string(), "/usr/src/app.jar".to_string());

        let result = variables.resolve_placeholders("{stem:jar_path}");
        assert_eq!(result.unwrap(), "app");
    }

    #[test]
    fn test_modifier_dir() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("jar_path".to_string(), "/usr/src/app.jar".to_string());

        let result = variables.resolve_placeholders("{dir:jar_path}");
        assert_eq!(result.unwrap(), "/usr/src");
    }

    #[test]
    fn test_modifier_ext() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("jar_path".to_string(), "/usr/src/app.jar".to_string());

        let result = variables.resolve_placeholders("{ext:jar_path}");
        assert_eq!(result.unwrap(), "jar");
    }

    #[test]
    fn test_modifier_abspath() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("rel_path".to_string(), "Cargo.toml".to_string());

        let result = variables.resolve_placeholders("{abspath:rel_path}");
        assert!(result.is_ok());
        let resolved = result.unwrap();
        // Should produce an absolute path
        assert!(
            std::path::Path::new(&resolved).is_absolute(),
            "Expected absolute path, got: {}",
            resolved
        );
    }

    // --- String modifier tests ---

    #[test]
    fn test_modifier_uppercase() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("svc".to_string(), "my_service".to_string());

        let result = variables.resolve_placeholders("{uppercase:svc}");
        assert_eq!(result.unwrap(), "MY_SERVICE");
    }

    #[test]
    fn test_modifier_lowercase() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("svc".to_string(), "My_Service".to_string());

        let result = variables.resolve_placeholders("{lowercase:svc}");
        assert_eq!(result.unwrap(), "my_service");
    }

    #[test]
    fn test_modifier_base64() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("token".to_string(), "my_secret_token".to_string());

        let result = variables.resolve_placeholders("{base64:token}");
        assert_eq!(result.unwrap(), "bXlfc2VjcmV0X3Rva2Vu");
    }

    #[test]
    fn test_modifier_trim() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("padded".to_string(), "  my_app  ".to_string());

        let result = variables.resolve_placeholders("{trim:padded}");
        assert_eq!(result.unwrap(), "my_app");
    }

    // --- Zero-input builtin tests ---

    #[test]
    fn test_builtin_hostname() {
        let variables = Variables::default();
        let result = variables.resolve_placeholders("{hostname}");
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_builtin_os() {
        let variables = Variables::default();
        let result = variables.resolve_placeholders("{os}");
        assert!(result.is_ok());
        let os = result.unwrap();
        assert!(
            os == "windows" || os == "linux" || os == "macos",
            "Unexpected OS: {}",
            os
        );
    }

    #[test]
    fn test_builtin_uuid_unique_per_occurrence() {
        // Given
        let variables = Variables::default();

        // When
        let result = variables.resolve_placeholders("{uuid}-{uuid}");

        // Then
        assert!(result.is_ok());
        let resolved = result.unwrap();
        // Format: UUID1-UUID2 where each UUID is 36 chars (8-4-4-4-12)
        assert_eq!(resolved.len(), 73, "Expected two UUIDs separated by dash, got: {}", resolved);
        let uuid1 = &resolved[..36];
        let uuid2 = &resolved[37..];
        assert_ne!(uuid1, uuid2, "Each {{uuid}} should produce a unique value");
    }

    #[test]
    fn test_builtin_now_default_format() {
        // Given
        let variables = Variables::default();

        // When
        let result = variables.resolve_placeholders("{now}");

        // Then
        assert!(result.is_ok());
        let now = result.unwrap();
        // ISO 8601 format: YYYY-MM-DDTHH:MM:SS+HH:MM (25 chars)
        assert!(now.len() >= 25, "Expected ISO 8601 format, got: {}", now);
        assert_eq!(now.chars().nth(4), Some('-'));
        assert_eq!(now.chars().nth(10), Some('T'));
    }

    #[test]
    fn test_builtin_now_resolved_per_occurrence() {
        let variables = Variables::default();

        let result = variables.resolve_placeholders("{now}|{now}");

        assert!(result.is_ok());
        let resolved = result.unwrap();
        let parts = resolved.split('|').collect::<Vec<_>>();
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|part| part.len() >= 25));
        assert!(parts.iter().all(|part| part.chars().nth(10) == Some('T')));
    }

    #[test]
    fn test_builtin_now_epoch() {
        // Given
        let variables = Variables::default();

        // When
        let result = variables.resolve_placeholders("{now:epoch}");

        // Then
        assert!(result.is_ok());
        let ts: i64 = result.unwrap().parse().expect("epoch should be numeric");
        assert!(ts > 1_000_000_000, "Epoch too small: {}", ts);
    }

    #[test]
    fn test_builtin_now_epoch_ms() {
        // Given
        let variables = Variables::default();

        // When
        let result = variables.resolve_placeholders("{now:epoch_ms}");

        // Then
        assert!(result.is_ok());
        let ts: i64 = result.unwrap().parse().expect("epoch_ms should be numeric");
        assert!(ts > 1_000_000_000_000, "Epoch ms too small: {}", ts);
    }

    #[test]
    fn test_builtin_now_custom_format() {
        // Given
        let variables = Variables::default();

        // When
        let result = variables.resolve_placeholders("{now:YYYY-MM-DD}");

        // Then
        assert!(result.is_ok());
        let date = result.unwrap();
        // YYYY-MM-DD = 10 chars
        assert_eq!(date.len(), 10);
        assert_eq!(date.chars().nth(4), Some('-'));
        assert_eq!(date.chars().nth(7), Some('-'));
    }

    #[test]
    fn test_builtin_now_custom_format_resolved_per_occurrence() {
        let variables = Variables::default();

        let result = variables.resolve_placeholders("{now:YYYY-MM-DD}|{now:YYYY-MM-DD}");

        assert!(result.is_ok());
        let resolved = result.unwrap();
        let parts = resolved.split('|').collect::<Vec<_>>();
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|part| part.len() == 10));
        assert!(parts.iter().all(|part| part.chars().nth(4) == Some('-')));
        assert!(parts.iter().all(|part| part.chars().nth(7) == Some('-')));
    }

    #[test]
    fn test_builtin_now_time_format() {
        // Given
        let variables = Variables::default();

        // When
        let result = variables.resolve_placeholders("{now:HHmmss}");

        // Then
        assert!(result.is_ok());
        let time = result.unwrap();
        assert_eq!(time.len(), 6);
        assert!(time.chars().all(|c| c.is_ascii_digit()));
    }

    // --- User variables override builtins ---

    #[test]
    fn test_user_variable_overrides_builtin() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("hostname".to_string(), "custom-host".to_string());

        let result = variables.resolve_placeholders("{hostname}");
        assert_eq!(result.unwrap(), "custom-host");
    }

    #[test]
    fn test_user_variable_overrides_builtin_now() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("now".to_string(), "fixed-time".to_string());

        let result = variables.resolve_placeholders("{now}");
        assert_eq!(result.unwrap(), "fixed-time");
    }

    // --- Combined modifier with other placeholders ---

    #[test]
    fn test_modifier_combined_with_regular_vars() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("jar_path".to_string(), "/opt/deploy/service.jar".to_string());
        variables
            .defined_mut()
            .insert("target_dir".to_string(), "/backup".to_string());

        let result =
            variables.resolve_placeholders("{target_dir}/{basename:jar_path}");
        assert_eq!(result.unwrap(), "/backup/service.jar");
    }

    #[test]
    fn test_multiple_modifiers_in_one_string() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("file".to_string(), "/usr/src/app.tar.gz".to_string());
        variables
            .defined_mut()
            .insert("svc".to_string(), "my_service".to_string());

        let result =
            variables.resolve_placeholders("{basename:file}-{uppercase:svc}");
        assert_eq!(result.unwrap(), "app.tar.gz-MY_SERVICE");
    }

    #[test]
    fn test_repeated_modifier_occurrences_are_each_resolved() {
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("file".to_string(), "/usr/src/app.tar.gz".to_string());

        let result = variables.resolve_placeholders("{basename:file}|{basename:file}");
        assert_eq!(result.unwrap(), "app.tar.gz|app.tar.gz");
    }

    #[test]
    fn test_modifier_abspath_nonexistent_falls_back_to_cwd_join() {
        // Given — non-existent file, canonicalize will fail
        let mut variables = Variables::default();
        variables
            .defined_mut()
            .insert("no_file".to_string(), "this_does_not_exist_xyz.txt".to_string());

        // When
        let result = variables.resolve_placeholders("{abspath:no_file}");

        // Then — fallback joins cwd + path
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(
            resolved.ends_with("this_does_not_exist_xyz.txt"),
            "Expected cwd-joined path, got: {}",
            resolved
        );
        assert!(
            std::path::Path::new(&resolved).is_absolute(),
            "Expected absolute path, got: {}",
            resolved
        );
    }
}
