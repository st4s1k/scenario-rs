//! Configuration for predefined variables with values set in config files.

use serde::Deserialize;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// Map of variable names to predefined values.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct DefinedVariablesConfig(HashMap<String, String>);

impl Deref for DefinedVariablesConfig {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DefinedVariablesConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<String, String>> for DefinedVariablesConfig {
    fn from(variables: HashMap<String, String>) -> Self {
        DefinedVariablesConfig(variables)
    }
}

impl DefinedVariablesConfig {
    /// Merges with `other`, where `other`'s values take precedence on conflicts.
    pub fn merge(&self, other: &DefinedVariablesConfig) -> DefinedVariablesConfig {
        let mut merged = self.0.clone();
        for (key, value) in &other.0 {
            merged.insert(key.clone(), value.clone());
        }
        DefinedVariablesConfig::from(merged)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::variables::defined::DefinedVariablesConfig;
    use std::collections::HashMap;

    #[test]
    fn test_defined_variables_config_default() {
        // Given & When
        let config = DefinedVariablesConfig::default();

        // Then
        assert!(config.is_empty());
    }

    #[test]
    fn test_defined_variables_config_from_hashmap() {
        // Given
        let mut variables = HashMap::new();
        variables.insert("key1".to_string(), "value1".to_string());
        variables.insert("key2".to_string(), "value2".to_string());

        // When
        let config = DefinedVariablesConfig::from(variables);

        // Then
        assert_eq!(config.len(), 2);
        assert_eq!(config.get("key1"), Some(&"value1".to_string()));
        assert_eq!(config.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_defined_variables_config_deref() {
        // Given
        let config = create_test_config();

        // When & Then
        assert_eq!(config.len(), 2);
        assert_eq!(config.get("username"), Some(&"admin".to_string()));
    }

    #[test]
    fn test_defined_variables_config_deref_mut() {
        // Given
        let mut config = create_test_config();

        // When
        config.insert("new_key".to_string(), "new_value".to_string());

        // Then
        assert_eq!(config.len(), 3);
        assert_eq!(config.get("new_key"), Some(&"new_value".to_string()));
    }

    #[test]
    fn test_defined_variables_config_merge() {
        // Given
        let config1 = create_test_config();

        let mut variables2 = HashMap::new();
        variables2.insert("username".to_string(), "superuser".to_string());
        variables2.insert("host".to_string(), "example.com".to_string());
        let config2 = DefinedVariablesConfig::from(variables2);

        // When
        let merged = config1.merge(&config2);

        // Then
        assert_eq!(merged.len(), 3);
        assert_eq!(merged.get("username"), Some(&"superuser".to_string()));
        assert_eq!(merged.get("app_dir"), Some(&"/opt/myapp".to_string()));
        assert_eq!(merged.get("host"), Some(&"example.com".to_string()));
    }

    #[test]
    fn test_defined_variables_config_clone() {
        // Given
        let original = create_test_config();

        // When
        let clone = original.clone();

        // Then
        assert_eq!(clone.len(), original.len());
        assert_eq!(clone.get("username"), original.get("username"));
        assert_eq!(clone.get("app_dir"), original.get("app_dir"));
    }

    #[test]
    fn test_defined_variables_config_debug() {
        // Given
        let config = create_test_config();

        // When
        let debug_str = format!("{:?}", config);

        // Then
        assert!(debug_str.contains("username"));
        assert!(debug_str.contains("admin"));
        assert!(debug_str.contains("app_dir"));
        assert!(debug_str.contains("/opt/myapp"));
    }

    // Test helpers
    fn create_test_config() -> DefinedVariablesConfig {
        let mut variables = HashMap::new();
        variables.insert("username".to_string(), "admin".to_string());
        variables.insert("app_dir".to_string(), "/opt/myapp".to_string());
        DefinedVariablesConfig::from(variables)
    }
}
