//! Configuration for defined variables in scenarios.
//!
//! This module provides configuration structures for predefined variables
//! that have values set in the scenario configuration files.

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::Deserialize;

/// Configuration for predefined variables in a scenario.
///
/// This struct represents a collection of variables with preset values defined
/// in the scenario configuration. These variables are available for use in the
/// scenario without needing to be provided at runtime.
///
/// # Examples
///
/// Creating an empty configuration:
///
/// ```
/// use scenario_rs_core::config::variables::defined::DefinedVariablesConfig;
///
/// let config = DefinedVariablesConfig::default();
/// assert!(config.is_empty());
/// ```
///
/// Creating a configuration with predefined variables:
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_core::config::variables::defined::DefinedVariablesConfig;
///
/// let mut variables = HashMap::new();
/// variables.insert("username".to_string(), "admin".to_string());
/// variables.insert("app_dir".to_string(), "/opt/myapp".to_string());
///
/// let config = DefinedVariablesConfig::from(variables);
/// assert_eq!(config.get("username"), Some(&"admin".to_string()));
/// assert_eq!(config.len(), 2);
/// ```
///
/// In a TOML configuration file:
/// ```toml
/// [variables.defined]
/// username = "admin"
/// app_dir = "/opt/myapp"
/// ```
#[derive(Deserialize, Clone, Debug, Default)]
pub struct DefinedVariablesConfig(HashMap<String, String>);

impl Deref for DefinedVariablesConfig {
    type Target = HashMap<String, String>;

    /// Dereferences to the underlying HashMap of variable name-value pairs.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DefinedVariablesConfig {
    /// Provides mutable access to the underlying HashMap of variable name-value pairs.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<String, String>> for DefinedVariablesConfig {
    /// Creates a `DefinedVariablesConfig` from a HashMap of variable name-value pairs.
    ///
    /// This constructor allows for the creation of a `DefinedVariablesConfig` from an existing
    /// HashMap, enabling flexibility in how variable configurations are initialized.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use scenario_rs_core::config::variables::defined::DefinedVariablesConfig;
    ///
    /// let mut variables = HashMap::new();
    /// variables.insert("hostname".to_string(), "example.com".to_string());
    /// variables.insert("port".to_string(), "8080".to_string());
    ///
    /// let config = DefinedVariablesConfig::from(variables);
    /// assert_eq!(config.get("hostname"), Some(&"example.com".to_string()));
    /// assert_eq!(config.get("port"), Some(&"8080".to_string()));
    /// ```
    ///
    /// # Arguments
    ///
    /// * `variables` - A HashMap containing variable names and their values
    fn from(variables: HashMap<String, String>) -> Self {
        DefinedVariablesConfig(variables)
    }
}

impl DefinedVariablesConfig {
    /// Merges this configuration with another, with other's values taking precedence.
    ///
    /// When a variable name exists in both configurations, the value from `other`
    /// overrides the value from this configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use scenario_rs_core::config::variables::defined::DefinedVariablesConfig;
    ///
    /// // Create first configuration
    /// let mut vars1 = HashMap::new();
    /// vars1.insert("username".to_string(), "admin".to_string());
    /// vars1.insert("port".to_string(), "8080".to_string());
    /// let config1 = DefinedVariablesConfig::from(vars1);
    ///
    /// // Create second configuration with overlapping variable
    /// let mut vars2 = HashMap::new();
    /// vars2.insert("username".to_string(), "superuser".to_string()); // Will override
    /// vars2.insert("host".to_string(), "example.com".to_string());   // Will be added
    /// let config2 = DefinedVariablesConfig::from(vars2);
    ///
    /// // Merge configurations
    /// let merged = config1.merge(&config2);
    ///
    /// // Check merged results
    /// assert_eq!(merged.get("username"), Some(&"superuser".to_string())); // From config2
    /// assert_eq!(merged.get("port"), Some(&"8080".to_string()));          // From config1
    /// assert_eq!(merged.get("host"), Some(&"example.com".to_string()));   // From config2
    /// ```
    ///
    /// # Arguments
    ///
    /// * `other` - The configuration to merge with this one
    ///
    /// # Returns
    ///
    /// A new configuration containing all variables from both configurations
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
    use super::*;

    // Test helpers
    fn create_test_config() -> DefinedVariablesConfig {
        let mut variables = HashMap::new();
        variables.insert("username".to_string(), "admin".to_string());
        variables.insert("app_dir".to_string(), "/opt/myapp".to_string());
        DefinedVariablesConfig::from(variables)
    }

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
}
