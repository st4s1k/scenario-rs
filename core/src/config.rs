use crate::scenario::errors::ScenarioConfigError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

#[derive(Deserialize, Clone, Debug)]
pub struct PartialScenarioConfig {
    pub import: Option<String>,
    pub credentials: Option<CredentialsConfig>,
    pub server: Option<ServerConfig>,
    pub execute: Option<ExecuteConfig>,
    pub variables: Option<PartialVariablesConfig>,
    pub tasks: Option<TasksConfig>,
}

impl PartialScenarioConfig {
    pub fn merge(&self, other: &PartialScenarioConfig) -> PartialScenarioConfig {
        PartialScenarioConfig {
            import: other.import.clone().or_else(|| self.import.clone()),
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

            if let Some(import_path_str) = &config.import {
                if visited_imports.contains(import_path_str) {
                    return Err(ScenarioConfigError::CircularImport(import_path_str.clone()));
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
            return Err(ScenarioConfigError::ImportNotFound(
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
            import: None,
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

#[derive(Deserialize, Clone, Debug)]
pub struct CredentialsConfig {
    pub username: String,
    pub password: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ExecuteConfig {
    pub steps: StepsConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StepsConfig(Vec<StepConfig>);

impl Deref for StepsConfig {
    type Target = Vec<StepConfig>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StepsConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct StepConfig {
    pub task: String,
    pub rollback: Option<RollbackStepsConfig>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RollbackStepsConfig(Vec<String>);

impl Deref for RollbackStepsConfig {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RollbackStepsConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct VariablesConfig {
    #[serde(default)]
    pub required: RequiredVariablesConfig,
    #[serde(default)]
    pub special: SpecialVariablesConfig,
    #[serde(default)]
    pub defined: DefinedVariablesConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PartialVariablesConfig {
    pub required: Option<RequiredVariablesConfig>,
    pub special: Option<SpecialVariablesConfig>,
    pub defined: Option<DefinedVariablesConfig>,
}

impl PartialVariablesConfig {
    pub fn merge(&self, other: &PartialVariablesConfig) -> PartialVariablesConfig {
        let mut merged_required = match (&self.required, &other.required) {
            (Some(self_req), Some(other_req)) => self_req.merge(other_req),
            (None, Some(req)) => req.clone(),
            (Some(req), None) => req.clone(),
            (None, None) => RequiredVariablesConfig::default(),
        };

        let merged_defined = match (&self.defined, &other.defined) {
            (Some(self_defined), Some(other_defined)) => self_defined.merge(other_defined),
            (None, Some(defined)) => defined.clone(),
            (Some(defined), None) => defined.clone(),
            (None, None) => DefinedVariablesConfig::default(),
        };

        for key in merged_defined.keys() {
            merged_required.remove(key);
        }

        PartialVariablesConfig {
            required: Some(merged_required),
            special: match (&self.special, &other.special) {
                (Some(self_special), Some(other_special)) => {
                    Some(self_special.merge(other_special))
                }
                (None, Some(special)) => Some(special.clone()),
                (Some(special), None) => Some(special.clone()),
                (None, None) => None,
            },
            defined: Some(merged_defined),
        }
    }
}

impl TryFrom<PartialVariablesConfig> for VariablesConfig {
    type Error = ScenarioConfigError;

    fn try_from(partial: PartialVariablesConfig) -> Result<Self, Self::Error> {
        Ok(VariablesConfig {
            required: partial.required.unwrap_or_default(),
            special: partial.special.unwrap_or_default(),
            defined: partial.defined.unwrap_or_default(),
        })
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct RequiredVariablesConfig(BTreeMap</* name */ String, /* label */ String>);

impl Deref for RequiredVariablesConfig {
    type Target = BTreeMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariablesConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RequiredVariablesConfig {
    pub fn merge(&self, other: &RequiredVariablesConfig) -> RequiredVariablesConfig {
        let mut merged = self.0.clone();
        for (key, value) in &other.0 {
            merged.insert(key.clone(), value.clone());
        }
        RequiredVariablesConfig(merged)
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct SpecialVariablesConfig(HashMap<String, String>);

impl Deref for SpecialVariablesConfig {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SpecialVariablesConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl SpecialVariablesConfig {
    pub fn merge(&self, other: &SpecialVariablesConfig) -> SpecialVariablesConfig {
        let mut merged = self.0.clone();
        for (key, value) in &other.0 {
            merged.insert(key.clone(), value.clone());
        }
        SpecialVariablesConfig(merged)
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct DefinedVariablesConfig(HashMap<String, String>);

impl Deref for DefinedVariablesConfig {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DefinedVariablesConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DefinedVariablesConfig {
    pub fn merge(&self, other: &DefinedVariablesConfig) -> DefinedVariablesConfig {
        let mut merged = self.0.clone();
        for (key, value) in &other.0 {
            merged.insert(key.clone(), value.clone());
        }
        DefinedVariablesConfig(merged)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct TasksConfig(HashMap<String, TaskConfig>);

impl Deref for TasksConfig {
    type Target = HashMap<String, TaskConfig>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TasksConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum TaskConfig {
    RemoteSudo {
        description: String,
        error_message: String,
        #[serde(flatten)]
        remote_sudo: RemoteSudoConfig,
    },
    SftpCopy {
        description: String,
        error_message: String,
        #[serde(flatten)]
        sftp_copy: SftpCopyConfig,
    },
}

#[derive(Deserialize, Clone, Debug)]
pub struct RemoteSudoConfig {
    pub command: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SftpCopyConfig {
    pub source_path: String,
    pub destination_path: String,
}
