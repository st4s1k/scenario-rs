//! Defines resolved variables for scenarios.
//!
//! This module provides types and implementations for managing resolved variables
//! that are used within scenarios

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// A collection of resolved variables for a scenario.
///
/// This struct wraps a HashMap of variable names to `ResolvedVariable` instances,
/// providing methods for managing these variables.
#[derive(Clone, Debug)]
pub struct ResolvedVariables(pub(crate) HashMap<String, String>);

impl Deref for ResolvedVariables {
    type Target = HashMap<String, String>;

    /// Dereferences to the underlying HashMap for read operations.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ResolvedVariables {
    /// Dereferences to the underlying HashMap for mutable operations.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for ResolvedVariables {
    /// Creates an empty collection of resolved variables.
    fn default() -> Self {
        ResolvedVariables(HashMap::new())
    }
}
