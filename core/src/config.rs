use crate::scenario::errors::ScenarioConfigError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::{
    collections::HashMap,
    fs::File,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

#[derive(Deserialize, Clone, Debug)]
pub struct ScenarioConfig {
    pub credentials: CredentialsConfig,
    pub server: ServerConfig,
    pub execute: ExecuteConfig,
    pub variables: VariablesConfig,
    pub tasks: TasksConfig,
}

impl TryFrom<PathBuf> for ScenarioConfig {
    type Error = ScenarioConfigError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let config_file: File = File::open(value)
            .map_err(ScenarioConfigError::CannotOpenFile)?;
        let config: ScenarioConfig = serde_json::from_reader(config_file)
            .map_err(ScenarioConfigError::CannotReadJson)?;
        Ok(config)
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
    pub rollback_steps: Option<RollbackStepsConfig>,
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

#[derive(Deserialize, Clone, Debug)]
pub struct VariablesConfig {
    pub required: RequiredVariablesConfig,
    pub special: SpecialVariablesConfig,
    pub defined: DefinedVariablesConfig,
}

#[derive(Deserialize, Clone, Debug)]
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

#[derive(Deserialize, Clone, Debug)]
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

#[derive(Deserialize, Clone, Debug)]
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
