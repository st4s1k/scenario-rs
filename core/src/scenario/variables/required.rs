//! Defines required variables for scenarios.
//!
//! This module provides types and implementations for managing required variables
//! that are used within scenarios.

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::config::variables::required::RequiredVariablesConfig;

/// A collection of required (runtime-provided) variables for a scenario.
#[derive(Clone, Debug, Default)]
pub struct RequiredVariables(HashMap<String, RequiredVariable>);

impl Deref for RequiredVariables {
    type Target = HashMap<String, RequiredVariable>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&RequiredVariablesConfig> for RequiredVariables {
    /// Creates `RequiredVariables` from config. Variables with a `default` are pre-filled.
    fn from(config: &RequiredVariablesConfig) -> Self {
        let mut required_variables = HashMap::<String, RequiredVariable>::new();

        for (name, var_config) in config.iter() {
            let value = var_config.default.clone().unwrap_or_default();

            required_variables.insert(
                name.clone(),
                RequiredVariable {
                    label: var_config.label.clone().unwrap_or_else(|| name.clone()),
                    value,
                    read_only: var_config.read_only,
                    file_picker: var_config.file_picker,
                },
            );
        }

        RequiredVariables(required_variables)
    }
}

impl RequiredVariables {
    /// Creates a map of variable names to their current values.
    pub fn value_map(&self) -> HashMap<String, String> {
        self.iter()
            .map(|(key, var)| (key.clone(), var.value.clone()))
            .collect()
    }

    /// Updates variables with new values. Only updates variables that already exist.
    pub fn upsert(&mut self, variables: HashMap<String, String>) {
        for (name, value) in variables {
            if let Some(required_variable) = self.get_mut(&name) {
                required_variable.value = value;
            }
        }
    }
}

/// Represents a single required variable with its metadata and value.
#[derive(Clone, Debug, Default)]
pub struct RequiredVariable {
    pub(crate) label: String,
    pub(crate) value: String,
    pub(crate) read_only: bool,
    pub(crate) file_picker: bool,
}

impl Deref for RequiredVariable {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl RequiredVariable {
    pub fn with_label(self, label: String) -> Self {
        RequiredVariable { label, ..self }
    }

    pub fn with_value(self, value: String) -> Self {
        RequiredVariable { value, ..self }
    }

    pub fn with_read_only(self, read_only: bool) -> Self {
        RequiredVariable { read_only, ..self }
    }

    pub fn with_file_picker(self, file_picker: bool) -> Self {
        RequiredVariable { file_picker, ..self }
    }

    /// Returns the user-friendly label for this variable.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the current value of this variable.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns whether this variable is read-only.
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub fn not_read_only(&self) -> bool {
        !self.read_only
    }

    /// Returns whether a file picker should be shown for this variable.
    pub fn file_picker(&self) -> bool {
        self.file_picker
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::variables::required::{RequiredVariableConfig, RequiredVariablesConfig},
        scenario::variables::required::{RequiredVariable, RequiredVariables},
    };
    use std::collections::HashMap;

    #[test]
    fn test_required_variable_getters() {
        // Given
        let variable = RequiredVariable {
            label: "Test Variable".to_string(),
            value: "test_value".to_string(),
            read_only: true,
            file_picker: false,
        };

        // When & Then
        assert_eq!(variable.label(), "Test Variable");
        assert_eq!(variable.value(), "test_value");
        assert!(variable.read_only());
        assert!(!variable.not_read_only());
    }

    #[test]
    fn test_required_variable_set_value() {
        // Given
        let mut variable = RequiredVariable {
            label: "Test Variable".to_string(),
            value: "initial_value".to_string(),
            read_only: false,
            file_picker: false,
        };

        // When
        variable.value = "new_value".to_string();

        // Then
        assert_eq!(variable.value(), "new_value");
    }

    #[test]
    fn test_from_config_with_defaults() {
        // Given
        let mut config_map = HashMap::new();
        config_map.insert(
            "username".to_string(),
            RequiredVariableConfig {
                label: Some("Username".to_string()),
                default: None,
                read_only: false,
                file_picker: false,
            },
        );
        config_map.insert(
            "deploy_time".to_string(),
            RequiredVariableConfig {
                label: Some("Deploy Time".to_string()),
                default: Some("{now:YYYY-MM-DDTHH:mm:ssZ}".to_string()),
                read_only: true,
                file_picker: false,
            },
        );
        config_map.insert(
            "unlabeled_var".to_string(),
            RequiredVariableConfig {
                label: None,
                default: None,
                read_only: false,
                file_picker: false,
            },
        );
        let config = RequiredVariablesConfig::from(config_map);

        // When
        let required_vars = RequiredVariables::from(&config);

        // Then
        assert_eq!(required_vars.len(), 3);

        let username = required_vars.get("username").unwrap();
        assert_eq!(username.label(), "Username");
        assert_eq!(username.value(), "");
        assert!(!username.read_only());

        let deploy_time = required_vars.get("deploy_time").unwrap();
        assert_eq!(deploy_time.label(), "Deploy Time");
        assert_eq!(deploy_time.value(), "{now:YYYY-MM-DDTHH:mm:ssZ}");
        assert!(deploy_time.read_only());

        let unlabeled = required_vars.get("unlabeled_var").unwrap();
        assert_eq!(unlabeled.label(), "unlabeled_var");
    }

    #[test]
    fn test_required_variables_default_and_empty_config() {
        // Given
        let empty_config = RequiredVariablesConfig::from(HashMap::new());

        // When
        let empty_vars = RequiredVariables::from(&empty_config);
        let default_vars = RequiredVariables::default();

        // Then
        assert!(empty_vars.is_empty());
        assert!(default_vars.is_empty());
    }

    #[test]
    fn test_required_variables_deref_and_deref_mut() {
        // Given
        let mut vars = RequiredVariables::default();
        vars.insert(
            "var1".to_string(),
            RequiredVariable {
                label: "Label 1".to_string(),
                value: "value1".to_string(),
                read_only: false,
                file_picker: false,
            },
        );

        // When & Then
        assert_eq!(vars.len(), 1);
        assert!(vars.contains_key("var1"));
        assert_eq!(vars.get("var1").unwrap().label(), "Label 1");
        assert_eq!(vars.get("var1").unwrap().value(), "value1");

        // When
        vars.insert(
            "var2".to_string(),
            RequiredVariable {
                label: "Label 2".to_string(),
                value: "value2".to_string(),
                read_only: false,
                file_picker: false,
            },
        );

        // Then
        assert_eq!(vars.len(), 2);
        let mut names = vars.keys().cloned().collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["var1", "var2"]);
    }

    #[test]
    fn test_required_variables_upsert() {
        // Given
        let mut vars = RequiredVariables::default();
        vars.insert(
            "string_var".to_string(),
            RequiredVariable {
                label: "String Var".to_string(),
                value: "original".to_string(),
                read_only: false,
                file_picker: false,
            },
        );
        vars.insert(
            "path_var".to_string(),
            RequiredVariable {
                label: "Path Variable".to_string(),
                value: "".to_string(),
                read_only: false,
                file_picker: false,
            },
        );

        let mut update_map = HashMap::new();
        update_map.insert("string_var".to_string(), "updated".to_string());
        update_map.insert("path_var".to_string(), "/tmp/test/file.txt".to_string());
        update_map.insert("nonexistent".to_string(), "ignored".to_string());

        // When
        vars.upsert(update_map);

        // Then
        assert_eq!(vars.get("string_var").unwrap().value(), "updated");
        assert_eq!(vars.get("path_var").unwrap().value(), "/tmp/test/file.txt");
        assert!(!vars.contains_key("nonexistent"));
    }

    #[test]
    fn test_required_variables_value_map() {
        // Given
        let mut vars = RequiredVariables::default();
        vars.insert(
            "var1".to_string(),
            RequiredVariable::default().with_value("value1".to_string()),
        );
        vars.insert(
            "var2".to_string(),
            RequiredVariable::default().with_value("value2".to_string()),
        );

        // When
        let map = vars.value_map();

        // Then
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("var1"), Some(&"value1".to_string()));
        assert_eq!(map.get("var2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_required_variable_not_read_only() {
        // Given
        let read_only_var = RequiredVariable::default().with_read_only(true);
        let writable_var = RequiredVariable::default().with_read_only(false);

        // When & Then
        assert!(!read_only_var.not_read_only());
        assert!(writable_var.not_read_only());
    }

    #[test]
    fn test_required_variable_builders() {
        // Given & When
        let var = RequiredVariable::default()
            .with_label("My Label".to_string())
            .with_value("my_val".to_string())
            .with_read_only(true)
            .with_file_picker(true);

        // Then
        assert_eq!(var.label(), "My Label");
        assert_eq!(var.value(), "my_val");
        assert!(var.read_only());
        assert!(var.file_picker());
    }

    #[test]
    fn test_required_variable_deref() {
        // Given
        let variable = RequiredVariable::default().with_value("hello".to_string());

        // When
        let deref_value: &String = &*variable;

        // Then
        assert_eq!(deref_value, "hello");
        assert_eq!(variable.len(), 5);
    }
}
