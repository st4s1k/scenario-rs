//! Configuration for required variables that must be provided at runtime.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// Map of variable names to their required variable configs.
#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct RequiredVariableConfig {
    /// User-facing label displayed in the TUI/GUI.
    #[serde(default)]
    pub label: Option<String>,
    /// Default value (supports placeholders like `{now:YYYY-MM-DD}`, `{uuid}`, etc.).
    #[serde(default)]
    pub default: Option<String>,
    /// If true, the user cannot edit this variable in the TUI/GUI.
    #[serde(default)]
    pub read_only: bool,
    /// If true, the TUI/GUI shows a file picker for this variable.
    #[serde(default)]
    pub file_picker: bool,
}

#[cfg(test)]
mod tests {
    use crate::config::variables::required::{RequiredVariableConfig, RequiredVariablesConfig};
    use std::collections::HashMap;
    use toml;

    #[test]
    fn test_required_variables_config_default() {
        let config = RequiredVariablesConfig::default();
        assert!(config.is_empty());
    }

    #[test]
    fn test_required_variables_config_deref() {
        let config = create_test_config();
        assert_eq!(config.len(), 2);
        assert!(config.contains_key("username"));
        assert!(config.contains_key("deploy_time"));
    }

    #[test]
    fn test_required_variables_config_deref_mut() {
        let mut config = create_test_config();
        config.insert(
            "new_var".to_string(),
            RequiredVariableConfig {
                label: Some("New".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(config.len(), 3);
    }

    #[test]
    fn test_required_variables_config_merge() {
        let config1 = create_test_config();

        let mut vars2 = HashMap::new();
        vars2.insert(
            "username".to_string(),
            RequiredVariableConfig {
                label: Some("Admin Name".to_string()),
                default: Some("root".to_string()),
                read_only: true,
                ..Default::default()
            },
        );
        vars2.insert(
            "version".to_string(),
            RequiredVariableConfig {
                label: Some("Version".to_string()),
                ..Default::default()
            },
        );
        let config2 = RequiredVariablesConfig(vars2);

        let merged = config1.merge(&config2);

        assert_eq!(merged.len(), 3);

        let username_var = merged.get("username").unwrap();
        assert_eq!(username_var.label, Some("Admin Name".to_string()));
        assert_eq!(username_var.default, Some("root".to_string()));
        assert!(username_var.read_only);

        assert!(merged.contains_key("deploy_time"));
        assert!(merged.contains_key("version"));
    }

    #[test]
    fn test_required_variable_config_minimal_deserialization() {
        let toml_str = r#"
            label = "Username"
        "#;

        let variable: RequiredVariableConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(variable.label, Some("Username".to_string()));
        assert_eq!(variable.default, None);
        assert!(!variable.read_only);
    }

    #[test]
    fn test_required_variable_config_full_deserialization() {
        let toml_str = r#"
            label = "Deploy Timestamp"
            default = "{now:YYYY-MM-DDTHH:mm:ssZ}"
            read_only = true
        "#;

        let variable: RequiredVariableConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(variable.label, Some("Deploy Timestamp".to_string()));
        assert_eq!(
            variable.default,
            Some("{now:YYYY-MM-DDTHH:mm:ssZ}".to_string())
        );
        assert!(variable.read_only);
    }

    #[test]
    fn test_required_variable_config_empty_deserialization() {
        let toml_str = "";
        let variable: RequiredVariableConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(variable.label, None);
        assert_eq!(variable.default, None);
        assert!(!variable.read_only);
    }

    #[test]
    fn test_required_variable_config_equality() {
        let a = RequiredVariableConfig {
            label: Some("Test".to_string()),
            default: Some("{uuid}".to_string()),
            read_only: true,
            ..Default::default()
        };
        let b = a.clone();
        assert_eq!(a, b);

        let c = RequiredVariableConfig {
            label: Some("Test".to_string()),
            read_only: true,
            ..Default::default()
        };
        assert_ne!(a, c);
    }

    #[test]
    fn test_required_variables_config_clone() {
        let original = create_test_config();
        let clone = original.clone();

        assert_eq!(clone.len(), original.len());
        for (key, value) in original.iter() {
            let cloned_value = clone.get(key).unwrap();
            assert_eq!(cloned_value.label, value.label);
            assert_eq!(cloned_value.default, value.default);
            assert_eq!(cloned_value.read_only, value.read_only);
        }
    }

    fn create_test_config() -> RequiredVariablesConfig {
        let mut variables = HashMap::new();
        variables.insert(
            "username".to_string(),
            RequiredVariableConfig {
                label: Some("Username".to_string()),
                ..Default::default()
            },
        );
        variables.insert(
            "deploy_time".to_string(),
            RequiredVariableConfig {
                label: Some("Deploy Time".to_string()),
                default: Some("{now:YYYY-MM-DDTHH:mm:ssZ}".to_string()),
                read_only: true,
                ..Default::default()
            },
        );
        RequiredVariablesConfig(variables)
    }
}
