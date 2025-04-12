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
/// # Example
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
        DefinedVariablesConfig(merged)
    }
}
