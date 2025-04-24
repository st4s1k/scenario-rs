use crate::config::step::StepConfig;
use serde::Deserialize;
use std::ops::{Deref, DerefMut};

/// Configuration for a sequence of steps in a scenario.
///
/// This struct represents an ordered collection of steps to be executed
/// as part of a scenario. It wraps a `Vec<StepConfig>` and provides
/// convenient access to the underlying vector through Deref and DerefMut.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use crate::config::{step::StepConfig, steps::StepsConfig};

    #[test]
    fn test_default_creates_empty_vector() {
        // Given
        let steps = StepsConfig::default();

        // Then
        assert!(steps.is_empty());
    }

    #[test]
    fn test_deref_provides_access_to_vector_methods() {
        // Given
        let step1 = create_test_step_config();
        let step2 = create_test_step_config();
        let steps = StepsConfig::from(vec![step1.clone(), step2.clone()]);

        // When & Then
        assert_eq!(steps.len(), 2);
        assert_eq!(&steps[0], &step1);
        assert_eq!(&steps[1], &step2);
    }

    #[test]
    fn test_deref_mut_allows_modification_of_vector() {
        // Given
        let step = create_test_step_config();
        let mut steps = StepsConfig::from(vec![step.clone()]);

        // When
        let new_step = create_test_step_config();
        steps.push(new_step.clone());

        // Then
        assert_eq!(steps.len(), 2);
        assert_eq!(&steps[1], &new_step);
    }

    #[test]
    fn test_from_creates_steps_from_vector() {
        // Given
        let step1 = create_test_step_config();
        let step2 = create_test_step_config();
        let vec = vec![step1.clone(), step2.clone()];

        // When
        let steps = StepsConfig::from(vec);

        // Then
        assert_eq!(steps.len(), 2);
        assert_eq!(&steps[0], &step1);
        assert_eq!(&steps[1], &step2);
    }

    #[test]
    fn test_clone_creates_identical_copy() {
        // Given
        let steps = StepsConfig::from(vec![create_test_step_config(), create_test_step_config()]);

        // When
        let cloned = steps.clone();

        // Then
        assert_eq!(steps, cloned);
    }

    #[test]
    fn test_partial_eq_compares_contents() {
        // Given
        let steps1 = StepsConfig::from(vec![create_test_step_config(), create_test_step_config()]);
        let steps2 = StepsConfig::from(vec![create_test_step_config()]);

        // When & Then
        assert_eq!(steps1, steps1.clone());
        assert_ne!(steps1, steps2);
        assert_eq!(StepsConfig::default(), StepsConfig::default());
    }

    #[test]
    fn test_debug_formatting() {
        // Given
        let steps = StepsConfig::default();

        // When
        let debug_str = format!("{:?}", steps);

        // Then
        assert!(debug_str.contains("StepsConfig"));
    }

    // Helper functions
    fn create_test_step_config() -> StepConfig {
        StepConfig::default()
    }
}
