use schemars::JsonSchema;
use serde::Deserialize;

/// A step context: references either a single task or a sequence, with optional on-fail fallback.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StepContext {
    /// Name identifying this step.
    pub name: String,
    /// Reference to a task name (mutually exclusive with `sequence`).
    pub task: Option<String>,
    /// Reference to a sequence name (mutually exclusive with `task`).
    pub sequence: Option<String>,
    /// Sequence name to execute as fallback on failure.
    #[serde(rename = "on-fail")]
    pub on_fail: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_context_task_deserialization() {
        let toml_str = r#"
            name = "deploy"
            task = "deploy_app"
            on-fail = "cleanup"
        "#;
        let step: StepContext = toml::from_str(toml_str).unwrap();
        assert_eq!(step.name, "deploy");
        assert_eq!(step.task.as_deref(), Some("deploy_app"));
        assert!(step.sequence.is_none());
        assert_eq!(step.on_fail.as_deref(), Some("cleanup"));
    }

    #[test]
    fn test_step_context_sequence_deserialization() {
        let toml_str = r#"
            name = "deploy"
            sequence = "full_deploy"
        "#;
        let step: StepContext = toml::from_str(toml_str).unwrap();
        assert!(step.task.is_none());
        assert_eq!(step.sequence.as_deref(), Some("full_deploy"));
        assert!(step.on_fail.is_none());
    }

    #[test]
    fn test_step_context_minimal() {
        let toml_str = r#"
            name = "simple_step"
            task = "simple"
        "#;
        let step: StepContext = toml::from_str(toml_str).unwrap();
        assert_eq!(step.task.as_deref(), Some("simple"));
        assert!(step.sequence.is_none());
        assert!(step.on_fail.is_none());
    }

    #[test]
    fn test_step_context_equality() {
        let step1 = StepContext {
            name: "s1".to_string(),
            task: Some("deploy".to_string()),
            sequence: None,
            on_fail: None,
        };
        let step2 = StepContext {
            name: "s1".to_string(),
            task: Some("deploy".to_string()),
            sequence: None,
            on_fail: None,
        };
        let step3 = StepContext {
            name: "s1".to_string(),
            task: None,
            sequence: Some("deploy".to_string()),
            on_fail: None,
        };
        assert_eq!(step1, step2);
        assert_ne!(step1, step3);
    }
}
