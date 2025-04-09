//! Variable configurations for scenarios.
//!
//! This module defines configuration structures for variables used in scenarios,
//! including both required variables (that must be provided at runtime) and
//! defined variables (with predefined values in the configuration).

use defined::DefinedVariablesConfig;
use required::RequiredVariablesConfig;
use serde::Deserialize;

use crate::scenario::errors::ScenarioConfigError;

pub mod defined;
pub mod required;

/// Complete configuration for variables in a scenario.
///
/// This struct holds configurations for both required variables (that must be 
/// provided at runtime) and defined variables (predefined in the configuration).
#[derive(Deserialize, Clone, Debug, Default)]
pub struct VariablesConfig {
    /// Configuration for variables that must be provided at runtime
    #[serde(default)]
    pub required: RequiredVariablesConfig,
    /// Configuration for variables with predefined values
    #[serde(default)]
    pub defined: DefinedVariablesConfig,
}

/// Partial configuration for variables that supports inheritance.
///
/// This structure represents an incomplete variables configuration that can be
    /// merged with another configuration, supporting hierarchical configuration.
#[derive(Deserialize, Clone, Debug)]
pub struct PartialVariablesConfig {
    /// Optional configuration for required variables
    pub required: Option<RequiredVariablesConfig>,
    /// Optional configuration for defined variables
    pub defined: Option<DefinedVariablesConfig>,
}

impl PartialVariablesConfig {
    /// Merges this configuration with another, with special handling for conflicts.
    ///
    /// When merging, defined variables take precedence over required variables.
    /// If a variable appears in both required and defined collections after merging,
    /// it will be removed from required since it already has a defined value.
    ///
    /// # Arguments
    ///
    /// * `other` - The configuration to merge with this one
    ///
    /// # Returns
    ///
    /// A new configuration that combines both configurations
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

    /// Converts a partial configuration into a complete configuration.
    ///
    /// This fills in any missing sections with defaults.
    ///
    /// # Returns
    ///
    /// * `Ok(VariablesConfig)` - A complete configuration with all sections present
    fn try_from(partial: PartialVariablesConfig) -> Result<Self, Self::Error> {
        Ok(VariablesConfig {
            required: partial.required.unwrap_or_default(),
            defined: partial.defined.unwrap_or_default(),
        })
    }
}
