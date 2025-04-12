//! Defines defined variables for scenarios.
//!
//! This module provides types and implementations for managing defined variables
//! that are used within scenarios

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::config::variables::defined::DefinedVariablesConfig;

/// A collection of defined variables for a scenario.
///
/// This struct wraps a HashMap of variable names to `DefinedVariable` instances,
/// providing methods for managing these variables.
///
/// # Examples
///
/// Creating defined variables:
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_core::scenario::variables::defined::DefinedVariables;
/// 
/// // Create an empty collection of defined variables
/// let mut defined = DefinedVariables::default();
/// 
/// // Add variables directly
/// defined.insert("host".to_string(), "example.com".to_string());
/// defined.insert("port".to_string(), "8080".to_string());
/// 
/// assert_eq!(defined.get("host").unwrap(), "example.com");
/// assert_eq!(defined.get("port").unwrap(), "8080");
/// ```
///
/// Creating from a configuration:
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_core::{
///     scenario::variables::defined::DefinedVariables,
/// };
/// 
/// // Create a HashMap for defined variables
/// let mut config_map = HashMap::new();
/// config_map.insert("env".to_string(), "production".to_string());
/// config_map.insert("app_name".to_string(), "my-service".to_string());
/// 
/// // Use From<HashMap<String, String>> to create DefinedVariables
/// let defined = DefinedVariables::from(config_map);
/// 
/// // Access variables
/// assert_eq!(defined.get("env").unwrap(), "production");
/// assert_eq!(defined.get("app_name").unwrap(), "my-service");
/// ```
#[derive(Clone, Debug)]
pub struct DefinedVariables(HashMap<String, String>);

impl Deref for DefinedVariables {
    type Target = HashMap<String, String>;

    /// Dereferences to the underlying HashMap for read operations.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DefinedVariables {
    /// Dereferences to the underlying HashMap for mutable operations.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&DefinedVariablesConfig> for DefinedVariables {
    /// Creates a `DefinedVariables` collection from a configuration.
    fn from(config: &DefinedVariablesConfig) -> Self {
        DefinedVariables(config.deref().clone())
    }
}

impl From<HashMap<String, String>> for DefinedVariables {
    /// Creates a `DefinedVariables` collection from a HashMap.
    fn from(map: HashMap<String, String>) -> Self {
        DefinedVariables(map)
    }
}

impl Default for DefinedVariables {
    /// Creates an empty collection of defined variables.
    fn default() -> Self {
        DefinedVariables(HashMap::new())
    }
}
