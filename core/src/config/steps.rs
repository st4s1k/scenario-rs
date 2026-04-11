use crate::config::step::StepContext;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Deserialize;
use std::ops::{Deref, DerefMut};

/// Ordered map of step names to their contexts, preserving TOML declaration order.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct StepsConfig(IndexMap<String, StepContext>);

impl Deref for StepsConfig {
    type Target = IndexMap<String, StepContext>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StepsConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<IndexMap<String, StepContext>> for StepsConfig {
    fn from(steps: IndexMap<String, StepContext>) -> Self {
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
            [first]
            task = "task_a"

            [second]
            task = "task_b"

            [third]
            sequence = "seq_c"
        "#;
        let steps: StepsConfig = toml::from_str(toml_str).unwrap();
        let keys: Vec<&String> = steps.keys().collect();
        assert_eq!(keys, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_steps_config_from_indexmap() {
        let mut map = IndexMap::new();
        map.insert(
            "step1".to_string(),
            StepContext {
                task: Some("task1".to_string()),
                sequence: None,
                on_fail: None,
            },
        );
        let steps = StepsConfig::from(map);
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_steps_config_with_on_fail() {
        let toml_str = r#"
            [deploy]
            task = "deploy_app"
            on-fail = "cleanup_sequence"
        "#;
        let steps: StepsConfig = toml::from_str(toml_str).unwrap();
        let step = steps.get("deploy").unwrap();
        assert_eq!(step.task.as_deref(), Some("deploy_app"));
        assert_eq!(step.on_fail.as_deref(), Some("cleanup_sequence"));
    }
}
