use crate::config::step::StepConfig;
use serde::Deserialize;
use std::ops::{Deref, DerefMut};

/// Configuration for a sequence of steps in a scenario.
///
/// This struct represents an ordered collection of steps to be executed
/// as part of a scenario. It wraps a `Vec<StepConfig>` and provides
/// convenient access to the underlying vector through Deref and DerefMut.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct StepsConfig(Vec<StepConfig>);

impl Deref for StepsConfig {
    type Target = Vec<StepConfig>;

    /// Dereferences to the underlying vector of step configurations.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StepsConfig {
    /// Provides mutable access to the underlying vector of step configurations.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<StepConfig>> for StepsConfig {
    /// Creates a new StepsConfig from a vector of StepConfig.
    fn from(steps: Vec<StepConfig>) -> Self {
        StepsConfig(steps)
    }
}
