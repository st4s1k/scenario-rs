//! Variable configurations: required (runtime) and defined (predefined values).

use crate::{
    config::variables::{defined::DefinedVariablesConfig, required::RequiredVariablesConfig},
    scenario::errors::ScenarioConfigError,
};
use schemars::JsonSchema;
use serde::Deserialize;

pub mod defined;
pub mod required;

/// Complete variables config: required + defined.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct VariablesConfig {
    #[serde(default)]
    pub required: RequiredVariablesConfig,
    #[serde(default)]
    pub defined: DefinedVariablesConfig,
}

/// Partial variables config supporting inheritance/merging.
#[derive(Deserialize, Clone, Debug, Default, JsonSchema)]
pub struct PartialVariablesConfig {
    pub required: Option<RequiredVariablesConfig>,
    pub defined: Option<DefinedVariablesConfig>,
}

impl PartialVariablesConfig {
    /// Merges with `other`. Defined variables take precedence: if a variable
    /// appears in both required and defined after merging, it's removed from required.
    pub fn merge(&self, other: &PartialVariablesConfig) -> PartialVariablesConfig {
        let mut merged_required = match (&self.required, &other.required) {
            (Some(self_req), Some(other_req)) => self_req.merge(other_req),
            (None, Some(req)) => req.clone(),
            (Some(req), None) => req.clone(),
            (None, None) => RequiredVariablesConfig::default(),
        };

        let merged_defined = match (&self.defined, &other.defined) {
            (Some(self_defined), Some(other_defined)) => self_defined.merge(other_defined),
            (None, Some(defined)) => defined.clone(),
            (Some(defined), None) => defined.clone(),
            (None, None) => DefinedVariablesConfig::default(),
        };

        for key in merged_defined.keys() {
            merged_required.remove(key);
        }

        PartialVariablesConfig {
            required: Some(merged_required),
            defined: Some(merged_defined),
        }
    }
}

impl TryFrom<PartialVariablesConfig> for VariablesConfig {
    type Error = ScenarioConfigError;

    fn try_from(partial: PartialVariablesConfig) -> Result<Self, Self::Error> {
        let required = match partial.required {
            Some(req) => req,
            None => RequiredVariablesConfig::default(),
        };

        let defined = match partial.defined {
            Some(def) => def,
            None => DefinedVariablesConfig::default(),
        };

        Ok(VariablesConfig { required, defined })
    }
}

#[cfg(test)]
mod tests {
    use crate::config::variables::{
        defined::DefinedVariablesConfig,
        required::{RequiredVariableConfig, RequiredVariablesConfig},
        PartialVariablesConfig, VariablesConfig,
    };
    use std::collections::HashMap;
    use toml;

    #[test]
    fn test_variables_config_default() {
        // Given & When
        let config = VariablesConfig::default();

        // Then
        assert!(config.required.is_empty());
        assert!(config.defined.is_empty());
    }

    #[test]
    fn test_variables_config_deserialization() {
        // Given
        let toml_str = r#"
            [required.username]
            label = "Username"

            [required.deploy_path]
            label = "Deployment Path"
            read_only = true

            [defined]
            environment = "production"
            port = "8080"
        "#;

        // When
        let config: VariablesConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(config.required.len(), 2);
        assert!(config.required.contains_key("username"));
        assert!(config.required.contains_key("deploy_path"));

        assert_eq!(config.defined.len(), 2);
        assert_eq!(
            config.defined.get("environment"),
            Some(&"production".to_string())
        );
        assert_eq!(config.defined.get("port"), Some(&"8080".to_string()));
    }

    #[test]
    fn test_partial_variables_config_default() {
        // Given & When
        let config = PartialVariablesConfig::default();

        // Then
        assert!(config.required.is_none());
        assert!(config.defined.is_none());
    }

    #[test]
    fn test_partial_variables_config_merge_both_empty() {
        // Given
        let config1 = PartialVariablesConfig::default();
        let config2 = PartialVariablesConfig::default();

        // When
        let merged = config1.merge(&config2);

        // Then
        assert!(merged.required.is_some());
        assert!(merged.defined.is_some());
        assert!(merged.required.unwrap().is_empty());
        assert!(merged.defined.unwrap().is_empty());
    }

    #[test]
    fn test_partial_variables_config_merge_one_empty() {
        // Given
        let config1 = PartialVariablesConfig::default();
        let config2 = create_test_partial_config();

        // When
        let merged = config1.merge(&config2);

        // Then
        assert!(merged.required.is_some());
        assert!(merged.defined.is_some());

        let required = merged.required.unwrap();
        assert_eq!(required.len(), 2);
        assert!(required.contains_key("username"));
        assert!(required.contains_key("config_path"));

        let defined = merged.defined.unwrap();
        assert_eq!(defined.len(), 2);
        assert_eq!(defined.get("environment"), Some(&"production".to_string()));
        assert_eq!(defined.get("log_level"), Some(&"info".to_string()));
    }

    #[test]
    fn test_partial_variables_config_merge_both_populated() {
        // Given
        let config1 = create_test_partial_config();

        let mut required_vars = HashMap::new();
        required_vars.insert(
            "username".to_string(),
            RequiredVariableConfig {
                label: Some("Admin Username".to_string()),
                read_only: true,
                ..Default::default()
            },
        );
        required_vars.insert(
            "api_key".to_string(),
            RequiredVariableConfig {
                label: Some("API Key".to_string()),
                ..Default::default()
            },
        );

        let mut defined_vars = HashMap::new();
        defined_vars.insert("environment".to_string(), "staging".to_string());
        defined_vars.insert("api_url".to_string(), "https://api.example.com".to_string());

        let config2 = PartialVariablesConfig {
            required: Some(RequiredVariablesConfig::from(required_vars)),
            defined: Some(DefinedVariablesConfig::from(defined_vars)),
        };

        // When
        let merged = config1.merge(&config2);

        // Then
        assert!(merged.required.is_some());
        assert!(merged.defined.is_some());

        let required = merged.required.unwrap();
        assert_eq!(required.len(), 3); // username (overridden), config_path, api_key

        let username_config = required.get("username").unwrap();
        assert_eq!(username_config.label, Some("Admin Username".to_string()));
        assert_eq!(username_config.read_only, true);
        assert!(required.contains_key("api_key"));

        let defined = merged.defined.unwrap();
        assert_eq!(defined.len(), 3); // environment (overridden), log_level, api_url
        assert_eq!(defined.get("environment"), Some(&"staging".to_string()));
        assert_eq!(defined.get("log_level"), Some(&"info".to_string()));
        assert_eq!(
            defined.get("api_url"),
            Some(&"https://api.example.com".to_string())
        );
    }

    #[test]
    fn test_partial_variables_config_merge_removes_defined_from_required() {
        // Given
        let mut required_vars = HashMap::new();
        required_vars.insert(
            "username".to_string(),
            RequiredVariableConfig {
                label: Some("Username".to_string()),
                ..Default::default()
            },
        );
        required_vars.insert(
            "environment".to_string(),
            RequiredVariableConfig {
                label: Some("Environment".to_string()),
                ..Default::default()
            },
        );

        let mut defined_vars = HashMap::new();
        defined_vars.insert("environment".to_string(), "production".to_string());

        let config1 = PartialVariablesConfig {
            required: Some(RequiredVariablesConfig::from(required_vars)),
            defined: None,
        };

        let config2 = PartialVariablesConfig {
            required: None,
            defined: Some(DefinedVariablesConfig::from(defined_vars)),
        };

        // When
        let merged = config1.merge(&config2);

        // Then
        let required = merged.required.unwrap();
        assert_eq!(required.len(), 1);
        assert!(required.contains_key("username"));
        assert!(!required.contains_key("environment"));

        let defined = merged.defined.unwrap();
        assert_eq!(defined.len(), 1);
        assert_eq!(defined.get("environment"), Some(&"production".to_string()));
    }

    #[test]
    fn test_partial_to_complete_conversion() {
        // Given
        let partial = create_test_partial_config();

        // When
        let result = VariablesConfig::try_from(partial);

        // Then
        assert!(result.is_ok());
        let config = result.unwrap();

        assert_eq!(config.required.len(), 2);
        assert!(config.required.contains_key("username"));
        assert!(config.required.contains_key("config_path"));

        assert_eq!(config.defined.len(), 2);
        assert_eq!(
            config.defined.get("environment"),
            Some(&"production".to_string())
        );
        assert_eq!(config.defined.get("log_level"), Some(&"info".to_string()));
    }

    #[test]
    fn test_partial_to_complete_with_empty_partial() {
        // Given
        let partial = PartialVariablesConfig::default();

        // When
        let result = VariablesConfig::try_from(partial);

        // Then
        assert!(result.is_ok());
        let config = result.unwrap();

        assert!(config.required.is_empty());
        assert!(config.defined.is_empty());
    }

    #[test]
    fn test_variables_config_clone() {
        // Given
        let mut required_vars = HashMap::new();
        required_vars.insert(
            "username".to_string(),
            RequiredVariableConfig {
                label: Some("Username".to_string()),
                ..Default::default()
            },
        );

        let mut defined_vars = HashMap::new();
        defined_vars.insert("environment".to_string(), "production".to_string());

        let original = VariablesConfig {
            required: RequiredVariablesConfig::from(required_vars),
            defined: DefinedVariablesConfig::from(defined_vars),
        };

        // When
        let clone = original.clone();

        // Then
        assert_eq!(clone, original);
    }

    #[test]
    fn test_partial_variables_config_debug() {
        // Given
        let config = create_test_partial_config();

        // When
        let debug_str = format!("{:?}", config);

        // Then
        assert!(debug_str.contains("required"));
        assert!(debug_str.contains("defined"));
        assert!(debug_str.contains("username"));
        assert!(debug_str.contains("environment"));
    }

    #[test]
    fn test_partial_variables_config_merge_self_defined_other_none() {
        // Given
        let config1 = PartialVariablesConfig {
            required: None,
            defined: Some(DefinedVariablesConfig::from(HashMap::from([
                ("port".to_string(), "8080".to_string()),
            ]))),
        };

        let config2 = PartialVariablesConfig {
            required: Some(RequiredVariablesConfig::from(HashMap::from([(
                "api_key".to_string(),
                RequiredVariableConfig {
                    label: Some("API Key".to_string()),
                    ..Default::default()
                },
            )]))),
            defined: None,
        };

        // When
        let merged = config1.merge(&config2);

        // Then
        let required = merged.required.unwrap();
        assert_eq!(required.len(), 1);
        assert!(required.contains_key("api_key"));

        let defined = merged.defined.unwrap();
        assert_eq!(defined.len(), 1);
        assert_eq!(defined.get("port"), Some(&"8080".to_string()));
    }

    fn create_test_partial_config() -> PartialVariablesConfig {
        let mut required_vars = HashMap::new();
        required_vars.insert(
            "username".to_string(),
            RequiredVariableConfig {
                label: Some("Username".to_string()),
                ..Default::default()
            },
        );
        required_vars.insert(
            "config_path".to_string(),
            RequiredVariableConfig {
                label: Some("Config Path".to_string()),
                read_only: true,
                file_picker: true,
                ..Default::default()
            },
        );

        let mut defined_vars = HashMap::new();
        defined_vars.insert("environment".to_string(), "production".to_string());
        defined_vars.insert("log_level".to_string(), "info".to_string());

        PartialVariablesConfig {
            required: Some(RequiredVariablesConfig::from(required_vars)),
            defined: Some(DefinedVariablesConfig::from(defined_vars)),
        }
    }
}
