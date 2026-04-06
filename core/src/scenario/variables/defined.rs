//! Defines defined variables for scenarios.
//!
//! This module provides types and implementations for managing defined variables
//! that are used within scenarios

use crate::config::variables::defined::DefinedVariablesConfig;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// A collection of defined (predefined in config) variables for a scenario.
#[derive(Clone, Debug)]
pub struct DefinedVariables(HashMap<String, String>);

impl Deref for DefinedVariables {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DefinedVariables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&DefinedVariablesConfig> for DefinedVariables {
    fn from(config: &DefinedVariablesConfig) -> Self {
        DefinedVariables(config.deref().clone())
    }
}

impl From<HashMap<String, String>> for DefinedVariables {
    fn from(map: HashMap<String, String>) -> Self {
        DefinedVariables(map)
    }
}

impl Default for DefinedVariables {
    fn default() -> Self {
        DefinedVariables(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::variables::defined::DefinedVariablesConfig,
        scenario::variables::defined::DefinedVariables,
    };
    use std::collections::HashMap;

    #[test]
    fn test_defined_variables_from_config() {
        // Given
        let mut config_map = HashMap::new();
        config_map.insert("env".to_string(), "production".to_string());
        config_map.insert("app".to_string(), "my-service".to_string());
        let config = DefinedVariablesConfig::from(config_map);

        // When
        let defined = DefinedVariables::from(&config);

        // Then
        assert_eq!(defined.len(), 2);
        assert_eq!(defined.get("env"), Some(&"production".to_string()));
        assert_eq!(defined.get("app"), Some(&"my-service".to_string()));
    }

    #[test]
    fn test_defined_variables_from_hashmap() {
        // Given
        let mut map = HashMap::new();
        map.insert("host".to_string(), "example.com".to_string());
        map.insert("port".to_string(), "8080".to_string());

        // When
        let defined = DefinedVariables::from(map);

        // Then
        assert_eq!(defined.len(), 2);
        assert_eq!(defined.get("host"), Some(&"example.com".to_string()));
        assert_eq!(defined.get("port"), Some(&"8080".to_string()));
    }

    #[test]
    fn test_defined_variables_default() {
        // Given & When
        let defined = DefinedVariables::default();

        // Then
        assert!(defined.is_empty());
    }

    #[test]
    fn test_defined_variables_deref() {
        // Given
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        let defined = DefinedVariables(map);

        // When & Then
        assert_eq!(defined.len(), 1);
        assert_eq!(defined.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_defined_variables_deref_mut() {
        // Given
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        let mut defined = DefinedVariables(map);

        // When
        defined.insert("key2".to_string(), "value2".to_string());
        defined.remove("key1");

        // Then
        assert_eq!(defined.len(), 1);
        assert_eq!(defined.get("key2"), Some(&"value2".to_string()));
        assert_eq!(defined.get("key1"), None);
    }

    #[test]
    fn test_defined_variables_clone() {
        // Given
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        let original = DefinedVariables(map);

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(cloned.len(), original.len());
        assert_eq!(cloned.get("key1"), original.get("key1"));
    }

    #[test]
    fn test_defined_variables_debug() {
        // Given
        let mut map = HashMap::new();
        map.insert("debug_key".to_string(), "debug_value".to_string());
        let defined = DefinedVariables(map);

        // When
        let debug_string = format!("{:?}", defined);

        // Then
        assert!(debug_string.contains("debug_key"));
        assert!(debug_string.contains("debug_value"));
    }
}
