use std::ops::{Deref, DerefMut};

use serde::Deserialize;

/// Configuration for fallback steps to execute when a scenario step fails.
///
/// This is a wrapper around a list of task names that should be executed
/// in sequence when the parent step encounters an error.
#[derive(Deserialize, Clone, Debug)]
pub struct OnFailStepsConfig(pub(crate) Vec<String>);

impl Deref for OnFailStepsConfig {
    type Target = Vec<String>;
    
    /// Dereferences to the underlying vector of task names.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OnFailStepsConfig {
    /// Provides mutable access to the underlying vector of task names.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
