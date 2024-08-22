use crate::scenario::errors::ScenarioConfigError;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::File,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

#[derive(Deserialize, Debug)]
pub struct ScenarioConfig {
    pub(crate) execute: ExecuteConfig,
    pub(crate) variables: VariablesConfig,
    pub(crate) tasks: TasksConfig,
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

#[derive(Deserialize, Debug)]
pub struct ExecuteConfig {
    pub(crate) steps: StepsConfig,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct StepConfig {
    pub(crate) task: String,
    pub(crate) rollback_steps: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct VariablesConfig {
    pub(crate) required: RequiredVariablesConfig,
    pub(crate) special: SpecialVariablesConfig,
    pub(crate) defined: DefinedVariablesConfig,
}

#[derive(Deserialize, Debug)]
pub struct RequiredVariablesConfig(Vec<String>);

impl Deref for RequiredVariablesConfig {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariablesConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct RemoteSudoConfig {
    pub(crate) command: String,
}

#[derive(Deserialize, Debug)]
pub struct SftpCopyConfig {
    pub(crate) source_path: String,
    pub(crate) destination_path: String,
}
