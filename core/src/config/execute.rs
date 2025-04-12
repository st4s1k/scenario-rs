use crate::config::steps::StepsConfig;
use serde::Deserialize;

/// Configuration for scenario execution.
///
/// This struct defines the execution flow of a scenario,
/// including the sequence of steps to be performed.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct ExecuteConfig {
    /// The ordered sequence of steps to execute in the scenario
    pub steps: StepsConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{on_fail::OnFailStepsConfig, step::StepConfig};
    use toml;

    // Test helpers at top of module
    fn create_test_config() -> ExecuteConfig {
        let steps = vec![
            StepConfig {
                task: "task1".to_string(),
                on_fail: None,
            },
            StepConfig {
                task: "task2".to_string(),
                on_fail: Some(OnFailStepsConfig::from(vec!["cleanup".to_string()])),
            },
        ];

        ExecuteConfig {
            steps: StepsConfig::from(steps),
        }
    }

    #[test]
    fn test_execute_config_creation() {
        // Given
        let config = create_test_config();

        // When & Then
        assert_eq!(config.steps.len(), 2);
        assert_eq!(config.steps[0].task, "task1");
        assert_eq!(config.steps[1].task, "task2");
        assert!(config.steps[1].on_fail.is_some());
    }

    #[test]
    fn test_execute_config_default() {
        // Given & When
        let config = ExecuteConfig::default();

        // Then
        assert_eq!(config.steps.len(), 0);
    }

    #[test]
    fn test_execute_config_clone() {
        // Given
        let original = create_test_config();

        // When
        let clone = original.clone();

        // Then
        assert_eq!(clone.steps.len(), original.steps.len());
        assert_eq!(clone.steps[0].task, original.steps[0].task);
        assert_eq!(clone.steps[1].task, original.steps[1].task);
    }

    #[test]
    fn test_execute_config_deserialization() {
        // Given
        let toml_str = r#"
            [[steps]]
            task = "task1"
            
            [[steps]]
            task = "task2"
            on-fail = ["cleanup"]
        "#;

        // When
        let config: ExecuteConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(config.steps.len(), 2);
        assert_eq!(config.steps[0].task, "task1");
        assert_eq!(config.steps[1].task, "task2");
        assert!(config.steps[1].on_fail.is_some());
        let on_fail = config.steps[1].on_fail.as_ref().unwrap();
        assert_eq!(on_fail.len(), 1);
        assert_eq!(on_fail[0], "cleanup");
    }
}
