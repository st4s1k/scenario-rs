use crate::{
    config::{
        credentials::{CredentialsConfig, PartialCredentialsConfig},
        sequences::SequencesConfig,
        server::{PartialServerConfig, ServerConfig},
        steps::StepsConfig,
        tasks::TasksConfig,
        variables::{PartialVariablesConfig, VariablesConfig},
    },
    scenario::errors::ScenarioConfigError,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;

/// Partial scenario config supporting inheritance via a `parent` field.
#[derive(Deserialize, Clone, Debug, JsonSchema)]
pub struct PartialScenarioConfig {
    pub parent: Option<String>,
    pub credentials: Option<PartialCredentialsConfig>,
    pub server: Option<PartialServerConfig>,
    pub steps: Option<StepsConfig>,
    pub sequences: Option<SequencesConfig>,
    pub variables: Option<PartialVariablesConfig>,
    pub tasks: Option<TasksConfig>,
}

impl PartialScenarioConfig {
    /// Merges with `other`. For most fields the other's value takes precedence;
    /// variables and tasks are merged recursively, steps are replaced wholesale.
    pub fn merge(&self, other: &PartialScenarioConfig) -> PartialScenarioConfig {
        PartialScenarioConfig {
            parent: other.parent.clone().or_else(|| self.parent.clone()),
            credentials: match (&self.credentials, &other.credentials) {
                (Some(self_creds), Some(other_creds)) => Some(self_creds.merge(other_creds)),
                (None, Some(creds)) => Some(creds.clone()),
                (Some(creds), None) => Some(creds.clone()),
                (None, None) => None,
            },
            server: match (&self.server, &other.server) {
                (Some(self_server), Some(other_server)) => Some(self_server.merge(other_server)),
                (None, Some(server)) => Some(server.clone()),
                (Some(server), None) => Some(server.clone()),
                (None, None) => None,
            },
            steps: other.steps.clone().or_else(|| self.steps.clone()),
            sequences: match (&self.sequences, &other.sequences) {
                (Some(base), Some(overrides)) => {
                    let mut merged = base.clone();
                    for (k, v) in overrides.iter() {
                        merged.insert(k.clone(), v.clone());
                    }
                    Some(merged)
                }
                (None, Some(seq)) => Some(seq.clone()),
                (Some(seq), None) => Some(seq.clone()),
                (None, None) => None,
            },
            variables: match (&self.variables, &other.variables) {
                (Some(self_vars), Some(other_vars)) => Some(self_vars.merge(other_vars)),
                (None, Some(vars)) => Some(vars.clone()),
                (Some(vars), None) => Some(vars.clone()),
                (None, None) => None,
            },
            tasks: match (&self.tasks, &other.tasks) {
                (Some(base), Some(overrides)) => Some(base.merge(overrides)),
                (None, Some(tasks)) => Some(tasks.clone()),
                (Some(tasks), None) => Some(tasks.clone()),
                (None, None) => None,
            },
        }
    }
}

/// Fully resolved scenario config with all required fields present.
#[derive(Deserialize, Clone, Debug, Default, JsonSchema)]
pub struct ScenarioConfig {
    pub credentials: CredentialsConfig,
    pub server: ServerConfig,
    pub steps: StepsConfig,
    #[serde(default)]
    pub sequences: SequencesConfig,
    pub variables: VariablesConfig,
    pub tasks: TasksConfig,
}

impl TryFrom<PartialScenarioConfig> for ScenarioConfig {
    type Error = ScenarioConfigError;

    fn try_from(partial: PartialScenarioConfig) -> Result<Self, Self::Error> {
        Ok(ScenarioConfig {
            credentials: match partial.credentials {
                Some(partial_creds) => CredentialsConfig::try_from(partial_creds)?,
                None => return Err(ScenarioConfigError::MissingCredentials),
            },
            server: match partial.server {
                Some(partial_server) => ServerConfig::try_from(partial_server)?,
                None => return Err(ScenarioConfigError::MissingServer),
            },
            steps: partial.steps.ok_or(ScenarioConfigError::MissingSteps)?,
            sequences: partial.sequences.unwrap_or_default(),
            variables: match partial.variables {
                Some(partial_vars) => VariablesConfig::try_from(partial_vars)?,
                None => VariablesConfig::default(),
            },
            tasks: partial.tasks.ok_or(ScenarioConfigError::MissingTasks)?,
        })
    }
}

impl ScenarioConfig {
    /// Follows parent references recursively, detecting circular dependencies.
    fn resolve_config_imports(
        initial_path: PathBuf,
    ) -> Result<Vec<PartialScenarioConfig>, ScenarioConfigError> {
        let mut visited_imports = Vec::new();
        let mut config_chain = Vec::new();
        let mut current_path = initial_path;

        loop {
            let config = Self::load_config_file(&current_path)?;

            if let Some(import_path_str) = &config.parent {
                if visited_imports.contains(import_path_str) {
                    return Err(ScenarioConfigError::CircularDependency(
                        import_path_str.clone(),
                    ));
                }

                visited_imports.push(import_path_str.clone());

                let import_path = Self::resolve_import_path(&current_path, import_path_str)?;

                config_chain.push(config);
                current_path = import_path;
            } else {
                config_chain.push(config);
                break;
            }
        }

        // Reverse to get base imports first (parent before child)
        config_chain.reverse();

        Ok(config_chain)
    }

    fn load_config_file(path: &PathBuf) -> Result<PartialScenarioConfig, ScenarioConfigError> {
        let config_string =
            std::fs::read_to_string(path).map_err(ScenarioConfigError::CannotOpenConfig)?;
        toml::from_str(&config_string).map_err(ScenarioConfigError::CannotReadConfig)
    }

    /// Resolves a relative or absolute import path against the current config's directory.
    fn resolve_import_path(
        current_config_path: &PathBuf,
        import_path_str: &str,
    ) -> Result<PathBuf, ScenarioConfigError> {
        let import_path = if std::path::Path::new(import_path_str).is_absolute() {
            PathBuf::from(import_path_str)
        } else {
            let parent_dir = current_config_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));

            parent_dir.join(import_path_str)
        };

        if !import_path.exists() {
            return Err(ScenarioConfigError::ParentConfigNotFound(
                import_path_str.to_string(),
            ));
        }

        Ok(import_path)
    }
}

impl TryFrom<PathBuf> for ScenarioConfig {
    type Error = ScenarioConfigError;

    fn try_from(config_path: PathBuf) -> Result<Self, Self::Error> {
        let config_dir = config_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();

        let configs_to_merge = Self::resolve_config_imports(config_path)?;

        let empty_config = PartialScenarioConfig {
            parent: None,
            credentials: None,
            server: None,
            steps: None,
            sequences: None,
            variables: None,
            tasks: None,
        };

        let merged_partial_config = configs_to_merge
            .iter()
            .fold(empty_config, |acc, config| acc.merge(config));

        let mut config = ScenarioConfig::try_from(merged_partial_config)?;

        if let Some(ref key_path) = config.credentials.private_key {
            let path = std::path::Path::new(key_path);
            if !path.is_absolute() {
                let resolved = config_dir.join(path);
                config.credentials.private_key =
                    Some(resolved.to_string_lossy().into_owned());
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            credentials::{CredentialsConfig, PartialCredentialsConfig},
            scenario::{PartialScenarioConfig, ScenarioConfig},
            server::{PartialServerConfig, ServerConfig},
            steps::StepsConfig,
            tasks::TasksConfig,
            variables::{PartialVariablesConfig, VariablesConfig},
        },
        scenario::errors::ScenarioConfigError,
    };

    #[test]
    fn test_partial_scenario_config_default() {
        // Given & When
        let partial = PartialScenarioConfig {
            parent: None,
            credentials: None,
            server: None,
            steps: None,
            sequences: None,
            variables: None,
            tasks: None,
        };

        assert!(partial.parent.is_none());
        assert!(partial.credentials.is_none());
        assert!(partial.server.is_none());
        assert!(partial.steps.is_none());
        assert!(partial.sequences.is_none());
        assert!(partial.variables.is_none());
        assert!(partial.tasks.is_none());
    }

    #[test]
    fn test_scenario_config_default() {
        // Given & When
        let config = ScenarioConfig::default();

        // Then
        assert_eq!(config.credentials, CredentialsConfig::default());
        assert_eq!(config.server, ServerConfig::default());
        assert_eq!(config.steps, StepsConfig::default());
        assert_eq!(config.variables, VariablesConfig::default());
        assert_eq!(config.tasks, TasksConfig::default());
    }

    #[test]
    fn test_partial_scenario_config_merge() {
        // Given
        let base = create_partial_base_config();
        let override_config = create_partial_override_config();

        // When
        let merged = base.merge(&override_config);

        // Then
        assert_eq!(merged.parent, Some("parent2.toml".to_string()));

        let merged_creds = merged.credentials.unwrap();
        assert_eq!(merged_creds.username, Some("user2".to_string()));
        assert_eq!(merged_creds.password, Some("pass1".to_string()));

        let merged_server = merged.server.unwrap();
        assert_eq!(merged_server.host, Some("host2".to_string()));
        assert_eq!(merged_server.port, Some(2222));

        assert!(merged.variables.is_some());
    }

    #[test]
    fn test_try_from_partial_scenario_config() {
        // Given
        let partial = create_full_partial_config();

        // When
        let result = ScenarioConfig::try_from(partial.clone());

        // Then
        assert!(result.is_ok());
        let complete = result.unwrap();

        assert_eq!(complete.credentials.username, "user".to_string());
        assert_eq!(complete.credentials.password, Some("pass".to_string()));

        assert_eq!(complete.server.host, "host".to_string());
        assert_eq!(complete.server.port, Some(22));

        assert_eq!(complete.steps, partial.steps.unwrap());
        assert_eq!(complete.tasks, partial.tasks.unwrap());
    }

    #[test]
    fn test_try_from_partial_scenario_config_missing_credentials() {
        // Given
        let mut partial = create_full_partial_config();
        partial.credentials = None;

        // When
        let result = ScenarioConfig::try_from(partial);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::MissingCredentials) => {}
            err => panic!("Expected MissingCredentials error, got {:?}", err),
        }
    }

    #[test]
    fn test_try_from_partial_scenario_config_missing_server() {
        // Given
        let mut partial = create_full_partial_config();
        partial.server = None;

        // When
        let result = ScenarioConfig::try_from(partial);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::MissingServer) => {}
            err => panic!("Expected MissingServer error, got {:?}", err),
        }
    }

    #[test]
    fn test_try_from_partial_scenario_config_missing_steps() {
        // Given
        let mut partial = create_full_partial_config();
        partial.steps = None;

        // When
        let result = ScenarioConfig::try_from(partial);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::MissingSteps) => {}
            err => panic!("Expected MissingSteps error, got {:?}", err),
        }
    }

    #[test]
    fn test_try_from_partial_scenario_config_missing_tasks() {
        // Given
        let mut partial = create_full_partial_config();
        partial.tasks = None;

        // When
        let result = ScenarioConfig::try_from(partial);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::MissingTasks) => {}
            err => panic!("Expected MissingTasks error, got {:?}", err),
        }
    }

    #[test]
    fn test_credential_field_conversion() {
        // Given
        let partial_creds = PartialCredentialsConfig {
            username: Some("test_user".to_string()),
            password: Some("test_pass".to_string()),
            private_key: None,
        };

        // When
        let creds = CredentialsConfig::try_from(partial_creds).unwrap();

        // Then
        assert_eq!(creds.username, "test_user");
        assert_eq!(creds.password, Some("test_pass".to_string()));
    }

    #[test]
    fn test_server_field_conversion() {
        // Given
        let partial_server = PartialServerConfig {
            host: Some("test_host".to_string()),
            port: Some(2222),
        };

        // When
        let server = ServerConfig::try_from(partial_server).unwrap();

        // Then
        assert_eq!(server.host, "test_host");
        assert_eq!(server.port, Some(2222));
    }

    #[test]
    fn test_partial_scenario_config_deserialization() {
        // Given
        let toml_str = r#"
            parent = "parent.toml"

            [credentials]
            username = "test_user"
            password = "test_pass"

            [server]
            host = "test_host"
            port = 2222

            [[steps]]
            name = "step1"
            task = "update"

            [[steps]]
            name = "step2"
            task = "restart"
            on-fail = "cleanup_seq"

            [sequences]
            cleanup_seq = ["rollback"]

            [tasks.remote_sudo.update]
            command = "apt-get update"
            description = "Update packages"

            [tasks.remote_sudo.restart]
            command = "systemctl restart app"

            [tasks.remote_sudo.rollback]
            command = "systemctl rollback app"
        "#;

        // When
        let config: PartialScenarioConfig = toml::from_str(toml_str).unwrap();

        // Then
        assert_eq!(config.parent, Some("parent.toml".to_string()));
        assert_eq!(
            config.credentials.as_ref().unwrap().username,
            Some("test_user".to_string())
        );
        assert_eq!(
            config.server.as_ref().unwrap().host,
            Some("test_host".to_string())
        );

        let steps = config.steps.as_ref().unwrap();
        assert_eq!(steps.len(), 2);
        let names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["step1", "step2"]);

        let tasks = config.tasks.as_ref().unwrap();
        assert_eq!(tasks.remote_sudo.as_ref().unwrap().len(), 3);
    }

    fn create_partial_base_config() -> PartialScenarioConfig {
        PartialScenarioConfig {
            parent: Some("parent1.toml".to_string()),
            credentials: Some(PartialCredentialsConfig {
                username: Some("user1".to_string()),
                password: Some("pass1".to_string()),
                private_key: None,
            }),
            server: Some(PartialServerConfig {
                host: Some("host1".to_string()),
                port: Some(1111),
            }),
            steps: Some(StepsConfig::default()),
            sequences: None,
            variables: Some(PartialVariablesConfig::default()),
            tasks: Some(TasksConfig::default()),
        }
    }

    fn create_partial_override_config() -> PartialScenarioConfig {
        PartialScenarioConfig {
            parent: Some("parent2.toml".to_string()),
            credentials: Some(PartialCredentialsConfig {
                username: Some("user2".to_string()),
                password: None,
                private_key: None,
            }),
            server: Some(PartialServerConfig {
                host: Some("host2".to_string()),
                port: Some(2222),
            }),
            steps: Some(StepsConfig::default()),
            sequences: None,
            variables: Some(PartialVariablesConfig::default()),
            tasks: Some(TasksConfig::default()),
        }
    }

    fn create_full_partial_config() -> PartialScenarioConfig {
        PartialScenarioConfig {
            parent: None,
            credentials: Some(PartialCredentialsConfig {
                username: Some("user".to_string()),
                password: Some("pass".to_string()),
                private_key: None,
            }),
            server: Some(PartialServerConfig {
                host: Some("host".to_string()),
                port: Some(22),
            }),
            steps: Some(StepsConfig::default()),
            sequences: None,
            variables: Some(PartialVariablesConfig::default()),
            tasks: Some(TasksConfig::default()),
        }
    }

    #[test]
    fn test_partial_scenario_merge_preserves_private_key() {
        // Given
        let base = PartialScenarioConfig {
            parent: None,
            credentials: Some(PartialCredentialsConfig {
                username: Some("user".to_string()),
                password: None,
                private_key: Some("/base/key".to_string()),
            }),
            server: Some(PartialServerConfig {
                host: Some("host".to_string()),
                port: Some(22),
            }),
            steps: Some(StepsConfig::default()),
            sequences: None,
            variables: Some(PartialVariablesConfig::default()),
            tasks: Some(TasksConfig::default()),
        };

        let child = PartialScenarioConfig {
            parent: None,
            credentials: Some(PartialCredentialsConfig {
                username: None,
                password: None,
                private_key: None,
            }),
            server: None,
            steps: None,
            sequences: None,
            variables: None,
            tasks: None,
        };

        // When
        let merged = base.merge(&child);

        // Then
        assert_eq!(
            merged.credentials.as_ref().unwrap().private_key,
            Some("/base/key".to_string())
        );
    }

    #[test]
    fn test_partial_scenario_config_deserialization_with_private_key() {
        // Given
        let toml_str = r#"
            steps = []

            [credentials]
            username = "key_user"
            private_key = "./my_key"

            [server]
            host = "host"
            port = 22

            [tasks]
        "#;

        // When
        let config: PartialScenarioConfig = toml::from_str(toml_str).unwrap();
        let creds = config.credentials.unwrap();

        // Then
        assert_eq!(creds.username, Some("key_user".to_string()));
        assert!(creds.password.is_none());
        assert_eq!(creds.private_key, Some("./my_key".to_string()));
    }

    #[test]
    fn test_try_from_pathbuf_single_config() {
        // Given
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("scenario.toml");
        std::fs::write(
            &config_path,
            r#"
            steps = []

            [credentials]
            username = "user"
            password = "pass"

            [server]
            host = "host"
            port = 22

            [tasks]
            "#,
        )
        .unwrap();

        // When
        let result = ScenarioConfig::try_from(config_path);

        // Then
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.credentials.username, "user");
        assert_eq!(config.server.host, "host");
    }

    #[test]
    fn test_try_from_pathbuf_with_parent() {
        // Given
        let dir = tempfile::tempdir().unwrap();
        let parent_path = dir.path().join("parent.toml");
        std::fs::write(
            &parent_path,
            r#"
            steps = []

            [credentials]
            username = "parent_user"
            password = "parent_pass"

            [server]
            host = "parent_host"
            port = 22

            [tasks]
            "#,
        )
        .unwrap();

        let child_path = dir.path().join("child.toml");
        std::fs::write(
            &child_path,
            r#"
            parent = "parent.toml"

            [credentials]
            username = "child_user"
            "#,
        )
        .unwrap();

        // When
        let result = ScenarioConfig::try_from(child_path);

        // Then
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.credentials.username, "child_user");
        assert_eq!(config.credentials.password, Some("parent_pass".to_string()));
        assert_eq!(config.server.host, "parent_host");
    }

    #[test]
    fn test_try_from_pathbuf_circular_dependency() {
        // Given
        let dir = tempfile::tempdir().unwrap();
        let a_path = dir.path().join("a.toml");
        let b_path = dir.path().join("b.toml");

        std::fs::write(&a_path, "parent = \"b.toml\"\n").unwrap();
        std::fs::write(&b_path, "parent = \"a.toml\"\n").unwrap();

        // When
        let result = ScenarioConfig::try_from(a_path);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::CircularDependency(path)) => {
                assert_eq!(path, "b.toml");
            }
            err => panic!("Expected CircularDependency error, got {:?}", err),
        }
    }

    #[test]
    fn test_try_from_pathbuf_parent_not_found() {
        // Given
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("scenario.toml");
        std::fs::write(&config_path, "parent = \"missing.toml\"\n").unwrap();

        // When
        let result = ScenarioConfig::try_from(config_path);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::ParentConfigNotFound(path)) => {
                assert_eq!(path, "missing.toml");
            }
            err => panic!("Expected ParentConfigNotFound error, got {:?}", err),
        }
    }

    #[test]
    fn test_try_from_pathbuf_cannot_open_config() {
        // Given
        let path = std::path::PathBuf::from("/nonexistent/path/config.toml");

        // When
        let result = ScenarioConfig::try_from(path);

        // Then
        assert!(matches!(result, Err(ScenarioConfigError::CannotOpenConfig(_))));
    }

    #[test]
    fn test_try_from_pathbuf_invalid_toml() {
        // Given
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("bad.toml");
        std::fs::write(&config_path, "this is not valid toml {{{").unwrap();

        // When
        let result = ScenarioConfig::try_from(config_path);

        // Then
        assert!(matches!(result, Err(ScenarioConfigError::CannotReadConfig(_))));
    }

    #[test]
    fn test_try_from_pathbuf_absolute_parent_path() {
        // Given
        let dir = tempfile::tempdir().unwrap();
        let parent_path = dir.path().join("parent.toml");
        std::fs::write(
            &parent_path,
            r#"
            steps = []

            [credentials]
            username = "user"
            password = "pass"

            [server]
            host = "host"
            port = 22

            [tasks]
            "#,
        )
        .unwrap();

        let child_path = dir.path().join("child.toml");
        let absolute_parent = parent_path.to_string_lossy().replace('\\', "/");
        std::fs::write(
            &child_path,
            format!("parent = \"{}\"\n", absolute_parent),
        )
        .unwrap();

        // When
        let result = ScenarioConfig::try_from(child_path);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_pathbuf_resolves_relative_private_key() {
        // Given
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("scenario.toml");
        std::fs::write(
            &config_path,
            r#"
            steps = []

            [credentials]
            username = "user"
            private_key = "./keys/id_rsa"

            [server]
            host = "host"
            port = 22

            [tasks]
            "#,
        )
        .unwrap();

        // When
        let result = ScenarioConfig::try_from(config_path);

        // Then
        assert!(result.is_ok());
        let config = result.unwrap();
        let key_path = config.credentials.private_key.unwrap();
        assert!(
            std::path::Path::new(&key_path).is_absolute(),
            "Relative private key should be resolved to absolute path"
        );
    }
}
