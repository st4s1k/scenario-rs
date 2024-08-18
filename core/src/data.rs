use crate::error::ScenarioConfigError;
use indicatif::ProgressBar;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    ops::{Deref, DerefMut},
    path::PathBuf,
};

pub struct Scenario {
    pub(crate) server: Server,
    pub(crate) credentials: Credentials,
    pub(crate) variables: Variables,
    pub(crate) config: ScenarioConfig,
}

#[derive(Deserialize, Debug)]
pub struct Server {
    pub(crate) host: String,
    pub(crate) port: String,
}

impl Server {
    pub fn new(host: &str, port: &str) -> Server {
        Server {
            host: host.to_string(),
            port: port.to_string(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Credentials {
    pub(crate) username: String,
    pub(crate) password: String,
}

impl Credentials {
    pub fn new(username: String, password: String) -> Credentials {
        Credentials { username, password }
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}

#[derive(Deserialize, Debug)]
pub struct ScenarioConfig {
    pub(crate) variables: VariablesConfig,
    pub(crate) steps: Vec<Step>,
}

impl TryFrom<PathBuf> for ScenarioConfig {
    type Error = ScenarioConfigError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let config_file: File = File::open(value)
            .map_err(ScenarioConfigError::CannotOpenFile)?;
        let config = serde_json::from_reader(config_file)
            .map_err(ScenarioConfigError::CannotReadJson)?;
        Ok(config)
    }
}

#[derive(Deserialize, Debug)]
pub struct RemoteSudo {
    pub(crate) command: String,
}

impl RemoteSudo {
    pub fn command(&self) -> &str {
        &self.command
    }
}

#[derive(Deserialize, Debug)]
pub struct SftpCopy {
    pub(crate) source_path: String,
    pub(crate) destination_path: String,
}

impl SftpCopy {
    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    pub fn destination_path(&self) -> &str {
        &self.destination_path
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Step {
    RemoteSudo {
        description: String,
        error_message: String,
        rollback_steps: Option<Vec<Step>>,
        #[serde(flatten)]
        remote_sudo: RemoteSudo,
    },
    SftpCopy {
        description: String,
        error_message: String,
        rollback_steps: Option<Vec<Step>>,
        #[serde(flatten)]
        sftp_copy: SftpCopy,
    },
}

impl Step {
    pub fn description(&self) -> &str {
        match self {
            Step::RemoteSudo { description, .. } => description,
            Step::SftpCopy { description, .. } => description,
        }
    }

    pub fn error_message(&self) -> &str {
        match self {
            Step::RemoteSudo { error_message, .. } => error_message,
            Step::SftpCopy { error_message, .. } => error_message,
        }
    }

    pub fn rollback_steps(&self) -> Option<&Vec<Step>> {
        match self {
            Step::RemoteSudo { rollback_steps, .. } => rollback_steps.as_ref(),
            Step::SftpCopy { rollback_steps, .. } => rollback_steps.as_ref(),
        }
    }
}

pub struct Variables(pub(crate) HashMap<String, String>);

impl Deref for Variables {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Variables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Deserialize, Debug)]
pub struct RequiredVariables(HashMap<String, String>);

impl RequiredVariables {
    pub fn new<const N: usize>(variables: [(String, String); N]) -> RequiredVariables {
        RequiredVariables(HashMap::from(variables))
    }
}

impl Deref for RequiredVariables {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Deserialize, Debug)]
pub struct VariablesConfig {
    pub(crate) required: RequiredVariablesConfig,
    pub(crate) defined: DefinedVariables,
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
pub struct DefinedVariables(HashMap<String, String>);

impl Deref for DefinedVariables {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DefinedVariables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct ExecutionLifecycle {
    pub before: fn(step: &Scenario),
    pub step: StepLifecycle,
}

impl Default for ExecutionLifecycle {
    fn default() -> Self {
        ExecutionLifecycle {
            before: |_| {},
            step: Default::default(),
        }
    }
}

pub struct StepLifecycle {
    pub before: fn(index: usize, step: &Step, steps: &Vec<Step>),
    pub remote_sudo: RemoteSudoLifecycle,
    pub sftp_copy: SftpCopyLifecycle,
    pub rollback: RollbackLifecycle,
}

impl Default for StepLifecycle {
    fn default() -> Self {
        StepLifecycle {
            before: |_, _, _| {},
            remote_sudo: Default::default(),
            sftp_copy: Default::default(),
            rollback: Default::default(),
        }
    }
}

pub struct RollbackLifecycle {
    pub before: fn(step: &Step),
    pub step: RollbackStepLifecycle,
}

impl Default for RollbackLifecycle {
    fn default() -> Self {
        RollbackLifecycle {
            before: |_| {},
            step: Default::default(),
        }
    }
}

pub struct RollbackStepLifecycle {
    pub before: fn(index: usize, rollback_step: &Step, rollback_steps: &Vec<Step>),
    pub remote_sudo: RemoteSudoLifecycle,
    pub sftp_copy: SftpCopyLifecycle,
}

impl Default for RollbackStepLifecycle {
    fn default() -> Self {
        RollbackStepLifecycle {
            before: |_, _, _| {},
            remote_sudo: Default::default(),
            sftp_copy: Default::default(),
        }
    }
}

pub struct RemoteSudoLifecycle {
    pub before: fn(remote_sudo: &RemoteSudo),
    pub channel_established: fn(channel_reader: &mut dyn Read),
}

impl Default for RemoteSudoLifecycle {
    fn default() -> Self {
        RemoteSudoLifecycle {
            before: |_| {},
            channel_established: |_| {},
        }
    }
}

pub struct SftpCopyLifecycle {
    pub before: fn(sftp_copy: &SftpCopy),
    pub files_ready: fn(source_file: &File, destination_writer: &mut dyn Write, pb: &ProgressBar),
    pub after: fn(),
}

impl Default for SftpCopyLifecycle {
    fn default() -> Self {
        SftpCopyLifecycle {
            before: |_| {},
            files_ready: |_, _, _| {},
            after: || {},
        }
    }
}
