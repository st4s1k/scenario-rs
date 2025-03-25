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

impl Default for DefinedVariables {
    /// Creates an empty collection of defined variables.
    fn default() -> Self {
        DefinedVariables(HashMap::new())
    }
}
