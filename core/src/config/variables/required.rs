//! Configuration for required variables that must be provided at runtime.

use serde::Deserialize;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// Map of variable names to their required variable configs.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct RequiredVariablesConfig(HashMap<String, RequiredVariableConfig>);

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

impl From<HashMap<String, RequiredVariableConfig>> for RequiredVariablesConfig {
    fn from(variables: HashMap<String, RequiredVariableConfig>) -> Self {
        RequiredVariablesConfig(variables)
    }
}

impl RequiredVariablesConfig {
    /// Merges with `other`, where `other` takes precedence on conflicts.
    pub fn merge(&self, other: &RequiredVariablesConfig) -> RequiredVariablesConfig {
        let mut merged = self.0.clone();
        for (key, value) in &other.0 {
            merged.insert(key.clone(), value.clone());
        }
        RequiredVariablesConfig(merged)
    }
}

/// Metadata for a single required variable.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct RequiredVariableConfig {
    #[serde(flatten)]
    pub var_type: VariableTypeConfig,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub read_only: bool,
}

/// Available types for required variables.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum VariableTypeConfig {
    #[default]
    String,
    /// Filesystem path — supports automatic basename extraction.
    Path,
    /// Initialized with the current time in the given format.
    Timestamp { format: String },
}

#[cfg(test)]
mod tests {
    use crate::config::variables::required::{
        RequiredVariableConfig, RequiredVariablesConfig, VariableTypeConfig,
    };
    use std::collections::HashMap;
    use toml;

    #[test]
    fn test_required_variables_config_default() {
        // Given & When
        let config = RequiredVariablesConfig::default();

        // Then
        assert!(config.is_empty());
    }

    #[test]
    fn test_required_variables_config_deref() {
        // Given
        let config = create_test_config();

        // When & Then
        assert_eq!(config.len(), 2);
        assert!(config.contains_key("username"));
        assert!(config.contains_key("config_path"));
    }

    #[test]
    fn test_required_variables_config_deref_mut() {
        // Given
        let mut config = create_test_config();

        // When
        config.insert("new_var".to_string(), create_test_timestamp_variable());

        // Then
        assert_eq!(config.len(), 3);
        assert!(config.contains_key("new_var"));
    }

    #[test]
    fn test_required_variables_config_merge() {
        // Given
        let config1 = create_test_config();

        let mut vars2 = HashMap::new();
        vars2.insert(
            "username".to_string(),
            RequiredVariableConfig {
                var_type: VariableTypeConfig::String,
                label: Some("Admin Name".to_string()),
                read_only: true,
            },
        );
        vars2.insert("timestamp".to_string(), create_test_timestamp_variable());
        let config2 = RequiredVariablesConfig(vars2);

        // When
        let merged = config1.merge(&config2);

        // Then
        assert_eq!(merged.len(), 3);

        let username_var = merged.get("username").unwrap();
        assert_eq!(username_var.label, Some("Admin Name".to_string()));
        assert_eq!(username_var.read_only, true);

        assert!(merged.contains_key("config_path"));

        assert!(merged.contains_key("timestamp"));
    }

    #[test]
    fn test_required_variable_config_deserialization() {
        // Given
        let toml_str = r#"
            type = "String"
            label = "Username"
            read_only = false
        "#;

        // When
        let variable: RequiredVariableConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(variable.var_type, VariableTypeConfig::String);
        assert_eq!(variable.label, Some("Username".to_string()));
        assert_eq!(variable.read_only, false);
    }

    #[test]
    fn test_required_variable_config_timestamp_deserialization() {
        // Given
        let toml_str = r#"
            type = "Timestamp"
            format = "%Y-%m-%d"
            label = "Release Date"
        "#;

        // When
        let variable: RequiredVariableConfig = toml::from_str(toml_str).unwrap();

        // Then
        match &variable.var_type {
            VariableTypeConfig::Timestamp { format } => {
                assert_eq!(format, "%Y-%m-%d");
            }
            _ => panic!("Expected Timestamp variable type"),
        }
        assert_eq!(variable.label, Some("Release Date".to_string()));
        assert_eq!(variable.read_only, false);
    }

    #[test]
    fn test_variable_type_config_equality() {
        // Given & When & Then
        assert_eq!(VariableTypeConfig::String, VariableTypeConfig::String);
        assert_eq!(VariableTypeConfig::Path, VariableTypeConfig::Path);
        assert_eq!(
            VariableTypeConfig::Timestamp {
                format: "%Y-%m-%d".to_string()
            },
            VariableTypeConfig::Timestamp {
                format: "%Y-%m-%d".to_string()
            }
        );

        assert_ne!(VariableTypeConfig::String, VariableTypeConfig::Path);
        assert_ne!(
            VariableTypeConfig::Timestamp {
                format: "%Y-%m-%d".to_string()
            },
            VariableTypeConfig::Timestamp {
                format: "%d/%m/%Y".to_string()
            }
        );
    }

    #[test]
    fn test_required_variables_config_clone() {
        // Given
        let original = create_test_config();

        // When
        let clone = original.clone();

        // Then
        assert_eq!(clone.len(), original.len());
        for (key, value) in original.iter() {
            let cloned_value = clone.get(key).unwrap();
            assert_eq!(cloned_value.label, value.label);
            assert_eq!(cloned_value.read_only, value.read_only);
        }
    }

    fn create_test_string_variable() -> RequiredVariableConfig {
        RequiredVariableConfig {
            var_type: VariableTypeConfig::String,
            label: Some("Username".to_string()),
            read_only: false,
        }
    }

    fn create_test_path_variable() -> RequiredVariableConfig {
        RequiredVariableConfig {
            var_type: VariableTypeConfig::Path,
            label: Some("Config Path".to_string()),
            read_only: true,
        }
    }

    fn create_test_timestamp_variable() -> RequiredVariableConfig {
        RequiredVariableConfig {
            var_type: VariableTypeConfig::Timestamp {
                format: "%Y-%m-%d".to_string(),
            },
            label: Some("Deployment Date".to_string()),
            read_only: false,
        }
    }

    fn create_test_config() -> RequiredVariablesConfig {
        let mut variables = HashMap::new();
        variables.insert("username".to_string(), create_test_string_variable());
        variables.insert("config_path".to_string(), create_test_path_variable());
        RequiredVariablesConfig(variables)
    }
}
