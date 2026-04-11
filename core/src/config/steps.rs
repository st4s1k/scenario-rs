use crate::config::step::StepContext;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Ordered list of steps, preserving TOML `[[steps]]` declaration order.
#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct StepsConfig(Vec<StepContext>);

impl Deref for StepsConfig {
    type Target = Vec<StepContext>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StepsConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<StepContext>> for StepsConfig {
    fn from(steps: Vec<StepContext>) -> Self {
        StepsConfig(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::step::StepContext;

    #[test]
    fn test_steps_config_default() {
        let steps = StepsConfig::default();
        assert!(steps.is_empty());
    }

    #[test]
    fn test_steps_config_preserves_order() {
        let toml_str = r#"
            [[step]]
            name = "first"
            task = "task_a"

            [[step]]
            name = "second"
            task = "task_b"

            [[step]]
            name = "third"
            sequence = "seq_c"
        "#;

        #[derive(Deserialize)]
        struct Wrapper {
            step: StepsConfig,
        }

        let wrapper: Wrapper = toml::from_str(toml_str).unwrap();
        let names: Vec<&str> = wrapper.step.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_steps_config_deref_mut() {
        let mut steps = StepsConfig::from(vec![StepContext {
            name: "step1".to_string(),
            task: Some("task1".to_string()),
            sequence: None,
            on_fail: None,
        }]);
        steps.push(StepContext {
            name: "step2".to_string(),
            task: Some("task2".to_string()),
            sequence: None,
            on_fail: None,
        });
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_steps_config_from_vec() {
        let steps = StepsConfig::from(vec![StepContext {
            name: "step1".to_string(),
            task: Some("task1".to_string()),
            sequence: None,
            on_fail: None,
        }]);
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_steps_config_with_on_fail() {
        let toml_str = r#"
            [[step]]
            name = "deploy"
            task = "deploy_app"
            on-fail = "cleanup_sequence"
        "#;

        #[derive(Deserialize)]
        struct Wrapper {
            step: StepsConfig,
        }

        let wrapper: Wrapper = toml::from_str(toml_str).unwrap();
        let step = &wrapper.step[0];
        assert_eq!(step.name, "deploy");
        assert_eq!(step.task.as_deref(), Some("deploy_app"));
        assert_eq!(step.on_fail.as_deref(), Some("cleanup_sequence"));
    }
}
