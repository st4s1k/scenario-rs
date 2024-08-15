use serde::Deserialize;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

pub struct Scenario {
    pub(crate) server: Server,
    pub(crate) credentials: Credentials,
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
    pub fn new(user: &str, password: &str) -> Credentials {
        Credentials {
            username: user.to_string(),
            password: password.to_string(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ScenarioConfig {
    pub(crate) variables: Variables,
    pub(crate) complete_message: Option<String>,
    pub(crate) steps: Vec<Step>,
}

#[derive(Deserialize, Debug)]
pub struct RemoteSudo {
    pub(crate) command: String,
}

#[derive(Deserialize, Debug)]
pub struct SftpCopy {
    pub(crate) source_path: String,
    pub(crate) destination_path: String,
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

#[derive(Deserialize, Debug)]
pub struct Variables(HashMap<String, String>);

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
