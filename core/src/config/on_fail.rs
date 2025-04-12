use serde::Deserialize;
use std::ops::{Deref, DerefMut};

/// Configuration for fallback steps to execute when a scenario step fails.
///
/// This is a wrapper around a list of task names that should be executed
/// in sequence when the parent step encounters an error.
///
/// # Examples
///
/// Creating a configuration with fallback tasks:
///
/// ```
/// use scenario_rs_core::config::on_fail::OnFailStepsConfig;
///
/// // Define fallback tasks to run in sequence after a failure
/// let cleanup_tasks = vec![
///     "remove_temp_files".to_string(),
///     "restore_backup".to_string(),
///     "notify_admin".to_string()
/// ];
///
/// let on_fail_config = OnFailStepsConfig::from(cleanup_tasks);
///
/// // Access the tasks using deref
/// assert_eq!(on_fail_config.len(), 3);
/// assert_eq!(on_fail_config[0], "remove_temp_files");
/// assert_eq!(on_fail_config[1], "restore_backup");
/// assert_eq!(on_fail_config[2], "notify_admin");
/// ```
///
/// In a TOML configuration file:
/// ```toml
/// [[steps]]
/// task = "deploy_application"
/// on_fail = ["cleanup_files", "restore_previous_version"]
/// ```
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct OnFailStepsConfig(Vec<String>);

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

impl From<Vec<String>> for OnFailStepsConfig {
    /// Creates a new `OnFailStepsConfig` from a vector of task names.
    fn from(tasks: Vec<String>) -> Self {
        OnFailStepsConfig(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    // Test helpers
    fn create_test_config() -> OnFailStepsConfig {
        OnFailStepsConfig(vec![
            "cleanup".to_string(),
            "notify".to_string(),
            "restore".to_string(),
        ])
    }
}
