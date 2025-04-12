//! Configuration for required variables in scenarios.
//!
//! This module provides configuration structures for variables that must be
//! provided by users at runtime, with metadata about their type and presentation.

use serde::Deserialize;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// Configuration for required variables in a scenario.
///
/// This struct represents a collection of variables that must be provided
/// by users at runtime. Each variable includes metadata about its type,
/// display label, and whether it's read-only.
///
/// # Examples
///
/// Creating an empty configuration:
///
/// ```
/// use scenario_rs_core::config::variables::required::RequiredVariablesConfig;
///
/// let config = RequiredVariablesConfig::default();
/// assert!(config.is_empty());
/// ```
///
/// Creating a configuration programmatically:
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_core::config::variables::required::{
///     RequiredVariablesConfig, RequiredVariableConfig, VariableTypeConfig
/// };
///
/// // Create configuration for a server IP variable
/// let server_ip_config = RequiredVariableConfig {
///     var_type: VariableTypeConfig::String,
///     label: Some("Server IP Address".to_string()),
///     read_only: false,
/// };
///
/// // Create configuration for a backup path variable
/// let backup_path_config = RequiredVariableConfig {
///     var_type: VariableTypeConfig::Path,
///     label: Some("Backup Directory".to_string()),
///     read_only: true,
/// };
///
/// // Create a HashMap of variables
/// let mut variables = HashMap::new();
/// variables.insert("server_ip".to_string(), server_ip_config);
/// variables.insert("backup_path".to_string(), backup_path_config);
///
/// // Create the configuration
/// let required_vars = RequiredVariablesConfig::from(variables);
///
/// assert_eq!(required_vars.len(), 2);
/// assert!(required_vars.contains_key("server_ip"));
/// assert!(required_vars.contains_key("backup_path"));
/// ```
///
/// In a TOML configuration file:
/// ```toml
/// [variables.required.server_ip]
/// type = "String"
/// label = "Server IP Address"
///
/// [variables.required.backup_path]
/// type = "Path"
/// label = "Backup Directory"
/// ```
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct RequiredVariablesConfig(HashMap<String, RequiredVariableConfig>);

impl Deref for RequiredVariablesConfig {
    type Target = HashMap<String, RequiredVariableConfig>;

    /// Dereferences to the underlying HashMap of variable name-config pairs.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariablesConfig {
    /// Provides mutable access to the underlying HashMap of variable name-config pairs.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<String, RequiredVariableConfig>> for RequiredVariablesConfig {
    /// Creates a new `RequiredVariablesConfig` from a HashMap of variable name-config pairs.
    ///
    /// This constructor allows for the creation of a `RequiredVariablesConfig` from an existing
    /// HashMap, enabling flexibility in how variable configurations are initialized.
    fn from(variables: HashMap<String, RequiredVariableConfig>) -> Self {
        RequiredVariablesConfig(variables)
    }
}

impl RequiredVariablesConfig {
    /// Merges this configuration with another, with other's values taking precedence.
    ///
    /// When a variable name exists in both configurations, the configuration from `other`
    /// overrides the configuration from this instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use scenario_rs_core::config::variables::required::{
    ///     RequiredVariablesConfig, RequiredVariableConfig, VariableTypeConfig
    /// };
    ///
    /// // Create first configuration
    /// let mut vars1 = HashMap::new();
    /// vars1.insert(
    ///     "username".to_string(),
    ///     RequiredVariableConfig {
    ///         var_type: VariableTypeConfig::String,
    ///         label: Some("Username".to_string()),
    ///         read_only: false,
    ///     }
    /// );
    /// let config1 = RequiredVariablesConfig::from(vars1);
    ///
    /// // Create second configuration
    /// let mut vars2 = HashMap::new();
    /// vars2.insert(
    ///     "username".to_string(),  // Will override existing
    ///     RequiredVariableConfig {
    ///         var_type: VariableTypeConfig::String,
    ///         label: Some("Admin Username".to_string()), // Different label
    ///         read_only: true, // Now read-only
    ///     }
    /// );
    /// vars2.insert(
    ///     "password".to_string(),  // New variable
    ///     RequiredVariableConfig {
    ///         var_type: VariableTypeConfig::String,
    ///         label: Some("Password".to_string()),
    ///         read_only: false,
    ///     }
    /// );
    /// let config2 = RequiredVariablesConfig::from(vars2);
    ///
    /// // Merge configurations
    /// let merged = config1.merge(&config2);
    ///
    /// assert_eq!(merged.len(), 2); // username (overridden) and password
    ///
    /// // Check the merged username config
    /// let username_config = merged.get("username").unwrap();
    /// assert_eq!(username_config.label, Some("Admin Username".to_string())); // From config2
    /// assert_eq!(username_config.read_only, true); // From config2
    ///
    /// // Verify password was added
    /// assert!(merged.contains_key("password"));
    /// ```
    ///
    /// # Arguments
    ///
    /// * `other` - The configuration to merge with this one
    ///
    /// # Returns
    ///
    /// A new configuration containing all variables from both configurations
    pub fn merge(&self, other: &RequiredVariablesConfig) -> RequiredVariablesConfig {
        let mut merged = self.0.clone();
        for (key, value) in &other.0 {
            merged.insert(key.clone(), value.clone());
        }
        RequiredVariablesConfig(merged)
    }
}

/// Configuration for a single required variable.
///
/// This struct defines metadata about a required variable, including its
/// type, display label, and whether it's read-only.
///
/// # Examples
///
/// Creating a string variable configuration:
///
/// ```
/// use scenario_rs_core::config::variables::required::{
///     RequiredVariableConfig, VariableTypeConfig
/// };
///
/// let config = RequiredVariableConfig {
///     var_type: VariableTypeConfig::String,
///     label: Some("API Key".to_string()),
///     read_only: true,
/// };
///
/// assert_eq!(config.label, Some("API Key".to_string()));
/// assert_eq!(config.read_only, true);
/// ```
///
/// Creating a timestamp variable configuration:
///
/// ```
/// use scenario_rs_core::config::variables::required::{
///     RequiredVariableConfig, VariableTypeConfig
/// };
///
/// let config = RequiredVariableConfig {
///     var_type: VariableTypeConfig::Timestamp {
///         format: "%Y-%m-%d %H:%M:%S".to_string()
///     },
///     label: Some("Deployment Time".to_string()),
///     read_only: false,
/// };
///
/// if let VariableTypeConfig::Timestamp { format } = &config.var_type {
///     assert_eq!(format, "%Y-%m-%d %H:%M:%S");
/// } else {
///     panic!("Expected Timestamp variable type");
/// }
/// ```
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct RequiredVariableConfig {
    /// The type of the variable (String, Path, or Timestamp)
    #[serde(flatten)]
    pub var_type: VariableTypeConfig,
    /// Optional human-friendly label for this variable
    #[serde(default)]
    pub label: Option<String>,
    /// Whether this variable can be changed after it's set
    #[serde(default)]
    pub read_only: bool,
}

/// Available types for required variables.
///
/// Different variable types have different behaviors and validation rules.
///
/// # Examples
///
/// Creating different variable types:
///
/// ```
/// use scenario_rs_core::config::variables::required::VariableTypeConfig;
///
/// // A simple string variable
/// let string_type = VariableTypeConfig::String;
///
/// // A path variable
/// let path_type = VariableTypeConfig::Path;
///
/// // A timestamp variable with a specific format
/// let timestamp_type = VariableTypeConfig::Timestamp {
///     format: "%Y-%m-%d".to_string()
/// };
/// ```
///
/// Comparing variable types:
///
/// ```
/// use scenario_rs_core::config::variables::required::VariableTypeConfig;
///
/// assert_eq!(VariableTypeConfig::String, VariableTypeConfig::String);
/// assert_ne!(VariableTypeConfig::String, VariableTypeConfig::Path);
/// ```
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum VariableTypeConfig {
    /// A simple text variable with no special handling
    #[default]
    String,
    /// A filesystem path with special handling for basename extraction
    Path,
    /// A timestamp that's initialized with the current time in the specified format
    Timestamp { format: String },
}

#[cfg(test)]
mod tests {
    use super::*;
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

        // Check that username was overridden
        let username_var = merged.get("username").unwrap();
        assert_eq!(username_var.label, Some("Admin Name".to_string()));
        assert_eq!(username_var.read_only, true);

        // Check that config_path was preserved
        assert!(merged.contains_key("config_path"));

        // Check that timestamp was added
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
        assert_eq!(variable.read_only, false); // Default value
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

    // Test helpers
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
