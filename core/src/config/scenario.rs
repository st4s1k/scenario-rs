use crate::{
    config::{
        credentials::{CredentialsConfig, PartialCredentialsConfig},
        execute::ExecuteConfig,
        server::{PartialServerConfig, ServerConfig},
        tasks::TasksConfig,
        variables::{PartialVariablesConfig, VariablesConfig},
    },
    scenario::errors::ScenarioConfigError,
};
use serde::Deserialize;
use std::path::PathBuf;

/// A partial scenario configuration that supports inheritance.
///
/// This structure represents an incomplete scenario configuration that can be
/// loaded from a TOML file. It allows for hierarchical configuration by
/// specifying a parent configuration file to inherit from.
#[derive(Deserialize, Clone, Debug)]
pub struct PartialScenarioConfig {
    /// Path to the parent configuration file, if any
    pub parent: Option<String>,
    /// Authentication credentials for the target server
    pub credentials: Option<PartialCredentialsConfig>,
    /// Server connection details
    pub server: Option<PartialServerConfig>,
    /// Execution configuration, including steps to run
    pub execute: Option<ExecuteConfig>,
    /// Definition of variables used in the scenario
    pub variables: Option<PartialVariablesConfig>,
    /// Tasks that can be executed as part of the scenario
    pub tasks: Option<TasksConfig>,
}

impl PartialScenarioConfig {
    /// Merges this configuration with another, with other's fields taking precedence.
    ///
    /// For most fields, if the other config has a value, it's used; otherwise,
    /// this config's value is used. For variables, the two configurations are
    /// merged recursively.
    ///
    /// # Arguments
    ///
    /// * `other` - The configuration to merge with this one
    ///
    /// # Returns
    ///
    /// A new configuration that combines both configurations
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
            execute: other.execute.clone().or_else(|| self.execute.clone()),
            variables: match (&self.variables, &other.variables) {
                (Some(self_vars), Some(other_vars)) => Some(self_vars.merge(other_vars)),
                (None, Some(vars)) => Some(vars.clone()),
                (Some(vars), None) => Some(vars.clone()),
                (None, None) => None,
            },
            tasks: other.tasks.clone().or_else(|| self.tasks.clone()),
        }
    }
}

/// A complete scenario configuration.
///
/// This represents a fully resolved scenario configuration with all required fields
/// present. It can be created by converting from a PartialScenarioConfig after
/// all inheritance has been resolved.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct ScenarioConfig {
    /// Authentication credentials for the target server
    pub credentials: CredentialsConfig,
    /// Server connection details
    pub server: ServerConfig,
    /// Execution configuration, including steps to run
    pub execute: ExecuteConfig,
    /// Definition of variables used in the scenario
    pub variables: VariablesConfig,
    /// Tasks that can be executed as part of the scenario
    pub tasks: TasksConfig,
}

impl TryFrom<PartialScenarioConfig> for ScenarioConfig {
    type Error = ScenarioConfigError;

    /// Converts a partial configuration into a complete configuration.
    ///
    /// Ensures all required fields are present and converts any partial
    /// sub-configurations into their complete versions.
    ///
    /// # Returns
    ///
    /// * `Ok(ScenarioConfig)` if all required fields are present
    /// * `Err` if any required field is missing
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
            execute: partial.execute.ok_or(ScenarioConfigError::MissingExecute)?,
            variables: match partial.variables {
                Some(partial_vars) => VariablesConfig::try_from(partial_vars)?,
                None => VariablesConfig::default(),
            },
            tasks: partial.tasks.ok_or(ScenarioConfigError::MissingTasks)?,
        })
    }
}

impl ScenarioConfig {
    /// Resolves the inheritance chain for a scenario configuration.
    ///
    /// Follows the parent references in each configuration file, loading them
    /// recursively until a configuration without a parent is found. Detects
    /// circular dependencies.
    ///
    /// # Arguments
    ///
    /// * `initial_path` - Path to the starting configuration file
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PartialScenarioConfig>)` if all configuration files were loaded successfully
    /// * `Err` if there was an error loading any configuration or a circular dependency was detected
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

    /// Loads a configuration file from disk.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file to load
    ///
    /// # Returns
    ///
    /// * `Ok(PartialScenarioConfig)` if the file was loaded and parsed successfully
    /// * `Err` if there was an error reading or parsing the file
    fn load_config_file(path: &PathBuf) -> Result<PartialScenarioConfig, ScenarioConfigError> {
        let config_string =
            std::fs::read_to_string(path).map_err(ScenarioConfigError::CannotOpenConfig)?;
        toml::from_str(&config_string).map_err(ScenarioConfigError::CannotReadConfig)
    }

    /// Resolves a relative or absolute import path.
    ///
    /// If the path is absolute, it's used as-is. If it's relative, it's
    /// interpreted relative to the directory containing the current config file.
    ///
    /// # Arguments
    ///
    /// * `current_config_path` - Path to the current configuration file
    /// * `import_path_str` - Path to the imported configuration file
    ///
    /// # Returns
    ///
    /// * `Ok(PathBuf)` with the resolved path if the file exists
    /// * `Err` if the file doesn't exist
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

    /// Creates a ScenarioConfig from a file path.
    ///
    /// Loads the configuration file and all its parent configurations,
    /// then merges them into a complete configuration.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// * `Ok(ScenarioConfig)` if the configuration was loaded and merged successfully
    /// * `Err` if there was an error loading or merging the configurations
    fn try_from(config_path: PathBuf) -> Result<Self, Self::Error> {
        let configs_to_merge = Self::resolve_config_imports(config_path)?;

        let empty_config = PartialScenarioConfig {
            parent: None,
            credentials: None,
            server: None,
            execute: None,
            variables: None,
            tasks: None,
        };

        let merged_partial_config = configs_to_merge
            .iter()
            .fold(empty_config, |acc, config| acc.merge(config));

        ScenarioConfig::try_from(merged_partial_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{execute::ExecuteConfig, task::TaskType, tasks::TasksConfig};

    #[test]
    fn test_partial_scenario_config_default() {
        // PartialScenarioConfig doesn't implement Default, so we create an empty one
        let partial = PartialScenarioConfig {
            parent: None,
            credentials: None,
            server: None,
            execute: None,
            variables: None,
            tasks: None,
        };

        assert!(partial.parent.is_none());
        assert!(partial.credentials.is_none());
        assert!(partial.server.is_none());
        assert!(partial.execute.is_none());
        assert!(partial.variables.is_none());
        assert!(partial.tasks.is_none());
    }

    #[test]
    fn test_scenario_config_default() {
        let config = ScenarioConfig::default();

        assert_eq!(config.credentials, CredentialsConfig::default());
        assert_eq!(config.server, ServerConfig::default());
        assert_eq!(config.execute, ExecuteConfig::default());
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

        // Check that credentials were merged
        let merged_creds = merged.credentials.unwrap();
        assert_eq!(merged_creds.username, Some("user2".to_string()));
        assert_eq!(merged_creds.password, Some("pass1".to_string())); // From base, not overridden

        // Check that server was merged
        let merged_server = merged.server.unwrap();
        assert_eq!(merged_server.host, Some("host2".to_string()));
        assert_eq!(merged_server.port, Some(2222));

        // Check that execute was overridden (not merged)
        assert_eq!(merged.execute, override_config.execute);

        // Check that variables were merged
        assert!(merged.variables.is_some());

        // Check that tasks were overridden (not merged)
        assert_eq!(merged.tasks, override_config.tasks);
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

        // Verify credentials conversion
        assert_eq!(complete.credentials.username, "user".to_string());
        assert_eq!(complete.credentials.password, Some("pass".to_string()));

        // Verify server conversion
        assert_eq!(complete.server.host, "host".to_string());
        assert_eq!(complete.server.port, Some(22));

        // Verify execute and tasks were copied as-is
        assert_eq!(complete.execute, partial.execute.unwrap());
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
            Err(ScenarioConfigError::MissingCredentials) => {} // expected
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
            Err(ScenarioConfigError::MissingServer) => {} // expected
            err => panic!("Expected MissingServer error, got {:?}", err),
        }
    }

    #[test]
    fn test_try_from_partial_scenario_config_missing_execute() {
        // Given
        let mut partial = create_full_partial_config();
        partial.execute = None;

        // When
        let result = ScenarioConfig::try_from(partial);

        // Then
        assert!(result.is_err());
        match result {
            Err(ScenarioConfigError::MissingExecute) => {} // expected
            err => panic!("Expected MissingExecute error, got {:?}", err),
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
            Err(ScenarioConfigError::MissingTasks) => {} // expected
            err => panic!("Expected MissingTasks error, got {:?}", err),
        }
    }

    #[test]
    fn test_credential_field_conversion() {
        // Given
        let partial_creds = PartialCredentialsConfig {
            username: Some("test_user".to_string()),
            password: Some("test_pass".to_string()),
        };

        // When
        let creds = match CredentialsConfig::try_from(partial_creds) {
            Ok(c) => c,
            Err(e) => panic!("Conversion failed: {:?}", e),
        };

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
        let server = match ServerConfig::try_from(partial_server) {
            Ok(s) => s,
            Err(e) => panic!("Conversion failed: {:?}", e),
        };

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
            
            [execute]
            steps = [
                { task = "task1" },
                { task = "task2", on_fail = ["cleanup"] }
            ]
            
            [tasks.task1]
            type = "RemoteSudo"
            description = "Test command description"
            command = "test_command1"
            error_message = "Test command error message"
            
            [tasks.task2]
            type = "RemoteSudo"
            description = "Another command description"
            command = "test_command2"
            error_message = "Another command error message"
            
            [tasks.cleanup]
            type = "RemoteSudo"
            description = "Cleanup command description"
            command = "cleanup_command"
            error_message = "Cleanup command error message"
        "#;

        // When
        let config: PartialScenarioConfig = match toml::from_str(toml_str) {
            Ok(c) => c,
            Err(e) => panic!("TOML parsing failed: {:?}", e),
        };

        // Then
        assert_eq!(config.parent, Some("parent.toml".to_string()));
        assert_eq!(
            config.credentials.as_ref().unwrap().username,
            Some("test_user".to_string())
        );
        assert_eq!(
            config.credentials.as_ref().unwrap().password,
            Some("test_pass".to_string())
        );
        assert_eq!(
            config.server.as_ref().unwrap().host,
            Some("test_host".to_string())
        );
        assert_eq!(config.server.as_ref().unwrap().port, Some(2222));

        // Verify task was parsed correctly
        let tasks = config.tasks.unwrap();
        assert!(tasks.contains_key("task1"));
        let task = &tasks["task1"];
        assert_eq!(task.description, "Test command description");
        assert_eq!(task.error_message, "Test command error message");
        match &task.task_type {
            TaskType::RemoteSudo { command } => {
                assert_eq!(command, "test_command1");
            }
            _ => panic!("Expected RemoteSudo task type"),
        }
    }

    // Helper functions to create test configs
    fn create_partial_base_config() -> PartialScenarioConfig {
        PartialScenarioConfig {
            parent: Some("parent1.toml".to_string()),
            credentials: Some(PartialCredentialsConfig {
                username: Some("user1".to_string()),
                password: Some("pass1".to_string()),
            }),
            server: Some(PartialServerConfig {
                host: Some("host1".to_string()),
                port: Some(1111),
            }),
            execute: Some(ExecuteConfig::default()),
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
            }),
            server: Some(PartialServerConfig {
                host: Some("host2".to_string()),
                port: Some(2222),
            }),
            execute: Some(ExecuteConfig::default()),
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
            }),
            server: Some(PartialServerConfig {
                host: Some("host".to_string()),
                port: Some(22),
            }),
            execute: Some(ExecuteConfig::default()),
            variables: Some(PartialVariablesConfig::default()),
            tasks: Some(TasksConfig::default()),
        }
    }
}
