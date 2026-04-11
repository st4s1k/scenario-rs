use crate::config::task::{RemoteSudoTaskConfig, SftpCopyTaskConfig};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;

/// Categorized task library: tasks organized by type.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct TasksConfig {
    pub remote_sudo: Option<HashMap<String, RemoteSudoTaskConfig>>,
    pub sftp_copy: Option<HashMap<String, SftpCopyTaskConfig>>,
}

impl TasksConfig {
    /// Merges another TasksConfig into this one. Entries from `other` override
    /// entries with the same name within each category.
    pub fn merge(&self, other: &TasksConfig) -> TasksConfig {
        TasksConfig {
            remote_sudo: match (&self.remote_sudo, &other.remote_sudo) {
                (Some(base), Some(overrides)) => {
                    let mut merged = base.clone();
                    merged.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
                    Some(merged)
                }
                (None, Some(tasks)) => Some(tasks.clone()),
                (Some(tasks), None) => Some(tasks.clone()),
                (None, None) => None,
            },
            sftp_copy: match (&self.sftp_copy, &other.sftp_copy) {
                (Some(base), Some(overrides)) => {
                    let mut merged = base.clone();
                    merged.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
                    Some(merged)
                }
                (None, Some(tasks)) => Some(tasks.clone()),
                (Some(tasks), None) => Some(tasks.clone()),
                (None, None) => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::task::{RemoteSudoTaskConfig, SftpCopyTaskConfig};

    #[test]
    fn test_tasks_config_default() {
        let tasks = TasksConfig::default();
        assert!(tasks.remote_sudo.is_none());
        assert!(tasks.sftp_copy.is_none());
    }

    #[test]
    fn test_tasks_config_deserialization() {
        let toml_str = r#"
            [remote_sudo.update]
            command = "apt-get update"

            [remote_sudo.restart]
            command = "systemctl restart app"

            [sftp_copy.deploy_config]
            source = "/local/config.json"
            destination = "/remote/config.json"
        "#;
        let tasks: TasksConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(tasks.remote_sudo.as_ref().unwrap().len(), 2);
        assert_eq!(tasks.sftp_copy.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_tasks_config_merge() {
        let base = TasksConfig {
            remote_sudo: Some(HashMap::from([(
                "task1".into(),
                RemoteSudoTaskConfig {
                    command: "echo 1".into(),
                    ..Default::default()
                },
            )])),
            sftp_copy: None,
        };
        let other = TasksConfig {
            remote_sudo: Some(HashMap::from([(
                "task2".into(),
                RemoteSudoTaskConfig {
                    command: "echo 2".into(),
                    ..Default::default()
                },
            )])),
            sftp_copy: Some(HashMap::from([(
                "copy1".into(),
                SftpCopyTaskConfig {
                    source: "a".into(),
                    destination: "b".into(),
                    ..Default::default()
                },
            )])),
        };
        let merged = base.merge(&other);
        assert_eq!(merged.remote_sudo.as_ref().unwrap().len(), 2);
        assert_eq!(merged.sftp_copy.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_tasks_config_merge_override() {
        let base = TasksConfig {
            remote_sudo: Some(HashMap::from([(
                "task1".into(),
                RemoteSudoTaskConfig {
                    command: "original".into(),
                    ..Default::default()
                },
            )])),
            sftp_copy: None,
        };
        let other = TasksConfig {
            remote_sudo: Some(HashMap::from([(
                "task1".into(),
                RemoteSudoTaskConfig {
                    command: "overridden".into(),
                    ..Default::default()
                },
            )])),
            sftp_copy: None,
        };
        let merged = base.merge(&other);
        let task = merged.remote_sudo.as_ref().unwrap().get("task1").unwrap();
        assert_eq!(task.command, "overridden");
    }
}
