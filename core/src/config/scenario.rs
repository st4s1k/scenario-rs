use std::path::PathBuf;

use serde::Deserialize;

use crate::scenario::errors::ScenarioConfigError;

use super::{
    credentials::CredentialsConfig,
    execute::ExecuteConfig,
    server::ServerConfig,
    tasks::TasksConfig,
    variables::{PartialVariablesConfig, VariablesConfig},
};

#[derive(Deserialize, Clone, Debug)]
pub struct PartialScenarioConfig {
    pub parent: Option<String>,
    pub credentials: Option<CredentialsConfig>,
    pub server: Option<ServerConfig>,
    pub execute: Option<ExecuteConfig>,
    pub variables: Option<PartialVariablesConfig>,
    pub tasks: Option<TasksConfig>,
}

impl PartialScenarioConfig {
    pub fn merge(&self, other: &PartialScenarioConfig) -> PartialScenarioConfig {
        PartialScenarioConfig {
            parent: other.parent.clone().or_else(|| self.parent.clone()),
            credentials: other
                .credentials
                .clone()
                .or_else(|| self.credentials.clone()),
            server: other.server.clone().or_else(|| self.server.clone()),
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

#[derive(Deserialize, Clone, Debug)]
pub struct ScenarioConfig {
    pub credentials: CredentialsConfig,
    pub server: ServerConfig,
    pub execute: ExecuteConfig,
    pub variables: VariablesConfig,
    pub tasks: TasksConfig,
}

impl TryFrom<PartialScenarioConfig> for ScenarioConfig {
    type Error = ScenarioConfigError;

    fn try_from(partial: PartialScenarioConfig) -> Result<Self, Self::Error> {
        Ok(ScenarioConfig {
            credentials: partial
                .credentials
                .ok_or(ScenarioConfigError::MissingCredentials)?,
            server: partial.server.ok_or(ScenarioConfigError::MissingServer)?,
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
