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
