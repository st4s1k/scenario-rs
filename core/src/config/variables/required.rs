//! Configuration for required variables in scenarios.
//!
//! This module provides configuration structures for variables that must be
//! provided by users at runtime, with metadata about their type and presentation.

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::Deserialize;

/// Configuration for required variables in a scenario.
///
/// This struct represents a collection of variables that must be provided
/// by users at runtime. Each variable includes metadata about its type,
/// display label, and whether it's read-only.
///
/// # Example
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
#[derive(Deserialize, Clone, Debug, Default)]
pub struct RequiredVariablesConfig(pub HashMap<String, RequiredVariableConfig>);

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

impl RequiredVariablesConfig {
    /// Merges this configuration with another, with other's values taking precedence.
    ///
    /// When a variable name exists in both configurations, the configuration from `other`
    /// overrides the configuration from this instance.
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
#[derive(Deserialize, Clone, Debug)]
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
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum VariableTypeConfig {
    /// A simple text variable with no special handling
    String,
    /// A filesystem path with special handling for basename extraction
    Path,
    /// A timestamp that's initialized with the current time in the specified format
    Timestamp { format: String },
}
