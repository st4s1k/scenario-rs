use schemars::JsonSchema;
use serde::Deserialize;
use std::ops::{Deref, DerefMut};

/// Task names to execute as fallback when a step fails.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct OnFailStepsConfig(Vec<String>);

impl Deref for OnFailStepsConfig {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OnFailStepsConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<String>> for OnFailStepsConfig {
    fn from(tasks: Vec<String>) -> Self {
        OnFailStepsConfig(tasks)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::on_fail::OnFailStepsConfig;
    use serde::Deserialize;
    use toml;

    #[test]
    fn test_on_fail_steps_config_deref() {
        // Given
        let config = create_test_config();

        // When & Then
        assert_eq!(config.len(), 3);
        assert_eq!(config[0], "cleanup");
        assert_eq!(config[1], "notify");
        assert_eq!(config[2], "restore");
    }

    #[test]
    fn test_on_fail_steps_config_deref_mut() {
        // Given
        let mut config = create_test_config();

        // When
        config.push("log_error".to_string());
        config[0] = "cleanup_all".to_string();

        // Then
        assert_eq!(config.len(), 4);
        assert_eq!(config[0], "cleanup_all");
        assert_eq!(config[3], "log_error");
    }

    #[test]
    fn test_on_fail_steps_config_deserialization() {
        // Given
        #[derive(Deserialize)]
        struct Wrapper {
            on_fail: OnFailStepsConfig,
        }

        let toml_str = r#"on_fail = ["cleanup", "restore", "notify"]"#;

        // When
        let config: OnFailStepsConfig = toml::from_str::<Wrapper>(&toml_str).unwrap().on_fail;

        // Then
        assert_eq!(config.len(), 3);
        assert_eq!(config[0], "cleanup");
        assert_eq!(config[1], "restore");
        assert_eq!(config[2], "notify");
    }

    #[test]
    fn test_on_fail_steps_config_clone() {
        // Given
        let original = create_test_config();

        // When
        let clone = original.clone();

        // Then
        assert_eq!(clone.len(), original.len());
        for (i, task) in original.iter().enumerate() {
            assert_eq!(&clone[i], task);
        }
    }

    #[test]
    fn test_on_fail_steps_config_debug() {
        // Given
        let config = create_test_config();

        // When
        let debug_str = format!("{:?}", config);

        // Then
        assert!(debug_str.contains("cleanup"));
        assert!(debug_str.contains("notify"));
        assert!(debug_str.contains("restore"));
    }

    fn create_test_config() -> OnFailStepsConfig {
        OnFailStepsConfig(vec![
            "cleanup".to_string(),
            "notify".to_string(),
            "restore".to_string(),
        ])
    }
}
