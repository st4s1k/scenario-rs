use crate::config::on_fail::OnFailStepsConfig;
use serde::Deserialize;

/// Configuration for a single execution step in a scenario.
///
/// A step represents a single task to be performed as part of a scenario,
/// along with optional fallback steps to execute if the task fails.
#[derive(Clone, Debug, Deserialize, Default, PartialEq, Eq)]
pub struct StepConfig {
    /// The identifier of the task to execute in this step
    pub task: String,
    /// Optional steps to execute if this step's task fails
    #[serde(rename = "on-fail")]
    pub on_fail: Option<OnFailStepsConfig>,
}

#[cfg(test)]
mod tests {
    use crate::config::{on_fail::OnFailStepsConfig, step::StepConfig};
    use toml;

    #[test]
    fn test_step_config_creation() {
        // Given
        let step = StepConfig {
            task: "deploy_app".to_string(),
            on_fail: None,
        };

        // Then
        assert_eq!(step.task, "deploy_app");
        assert!(step.on_fail.is_none());
    }

    #[test]
    fn test_step_config_with_on_fail() {
        // Given
        let on_fail = OnFailStepsConfig::from(vec!["cleanup".to_string(), "notify".to_string()]);

        let step = StepConfig {
            task: "deploy_app".to_string(),
            on_fail: Some(on_fail),
        };

        // Then
        assert_eq!(step.task, "deploy_app");
        assert!(step.on_fail.is_some());
        let on_fail = step.on_fail.unwrap();
        assert_eq!(on_fail.len(), 2);
        assert_eq!(on_fail[0], "cleanup");
        assert_eq!(on_fail[1], "notify");
    }

    #[test]
    fn test_step_config_default() {
        // Given
        let step = StepConfig::default();

        // Then
        assert_eq!(step.task, "");
        assert!(step.on_fail.is_none());
    }

    #[test]
    fn test_step_config_equality() {
        // Given
        let step1 = StepConfig {
            task: "deploy_app".to_string(),
            on_fail: None,
        };

        let step2 = StepConfig {
            task: "deploy_app".to_string(),
            on_fail: None,
        };

        let step3 = StepConfig {
            task: "different_task".to_string(),
            on_fail: None,
        };

        // Then
        assert_eq!(step1, step2);
        assert_ne!(step1, step3);
    }

    #[test]
    fn test_step_config_deserialization() {
        // Given
        let toml_str = r#"
            task = "deploy_app"
            on-fail = ["cleanup", "notify"]
        "#;

        // When
        let step: StepConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(step.task, "deploy_app");
        assert!(step.on_fail.is_some());
        let on_fail = step.on_fail.unwrap();
        assert_eq!(on_fail.len(), 2);
        assert_eq!(on_fail[0], "cleanup");
        assert_eq!(on_fail[1], "notify");
    }

    #[test]
    fn test_step_config_without_on_fail_deserialization() {
        // Given
        let toml_str = r#"
            task = "deploy_app"
        "#;

        // When
        let step: StepConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(step.task, "deploy_app");
        assert!(step.on_fail.is_none());
    }

    #[test]
    fn test_step_config_clone() {
        // Given
        let on_fail = OnFailStepsConfig::from(vec!["cleanup".to_string()]);
        let original = StepConfig {
            task: "deploy_app".to_string(),
            on_fail: Some(on_fail),
        };

        // When
        let clone = original.clone();

        // Then
        assert_eq!(clone, original);
        assert_eq!(clone.task, "deploy_app");
        assert_eq!(clone.on_fail.unwrap()[0], "cleanup");
    }
}
