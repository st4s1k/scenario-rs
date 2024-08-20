use crate::data::config::ScenarioConfig;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

pub struct Scenario {
    pub(crate) server: Server,
    pub(crate) credentials: Credentials,
    pub(crate) variables: Variables,
    pub(crate) config: ScenarioConfig,
}

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

pub mod config {
    use crate::error::ScenarioConfigError;
    use serde::Deserialize;
    use std::{
        collections::HashMap,
        fs::File,
        ops::{Deref, DerefMut},
        path::PathBuf,
    };

    #[derive(Deserialize, Debug)]
    pub struct ScenarioConfig {
        pub(crate) variables: VariablesConfig,
        pub(crate) steps: StepsConfig,
        pub(crate) tasks: TasksConfig,
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
    pub struct VariablesConfig {
        pub(crate) required: RequiredVariablesConfig,
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
    pub struct RemoteSudoConfig {
        pub(crate) command: String,
    }
    impl RemoteSudoConfig {
        pub fn command(&self) -> &str {
            &self.command
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct SftpCopyConfig {
        pub(crate) source_path: String,
        pub(crate) destination_path: String,
    }

    impl SftpCopyConfig {
        pub fn source_path(&self) -> &str {
            &self.source_path
        }

        pub fn destination_path(&self) -> &str {
            &self.destination_path
        }
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

    impl StepConfig {
        pub fn rollback_steps(&self) -> Option<&Vec<String>> {
            self.rollback_steps.as_ref()
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

    impl TaskConfig {
        pub fn description(&self) -> &str {
            match self {
                TaskConfig::RemoteSudo { description, .. } => description,
                TaskConfig::SftpCopy { description, .. } => description,
            }
        }

        pub fn error_message(&self) -> &str {
            match self {
                TaskConfig::RemoteSudo { error_message, .. } => error_message,
                TaskConfig::SftpCopy { error_message, .. } => error_message,
            }
        }
    }
}

pub mod lifecycles {
    use crate::data::config::StepConfig;
    use crate::data::{
        config::{
            RemoteSudoConfig,
            SftpCopyConfig,
            TaskConfig,
        },
        Scenario,
    };
    use indicatif::ProgressBar;
    use std::{
        fs::File,
        io::{Read, Write},
    };

    pub struct ExecutionLifecycle {
        pub before: fn(task: &Scenario),
        pub task: TaskLifecycle,
    }

    impl Default for ExecutionLifecycle {
        fn default() -> Self {
            ExecutionLifecycle {
                before: |_| {},
                task: Default::default(),
            }
        }
    }

    pub struct TaskLifecycle {
        pub before: fn(index: usize, task: &TaskConfig, tasks: Vec<&TaskConfig>),
        pub remote_sudo: RemoteSudoLifecycle,
        pub sftp_copy: SftpCopyLifecycle,
        pub rollback: RollbackLifecycle,
    }

    impl Default for TaskLifecycle {
        fn default() -> Self {
            TaskLifecycle {
                before: |_, _, _| {},
                remote_sudo: Default::default(),
                sftp_copy: Default::default(),
                rollback: Default::default(),
            }
        }
    }

    pub struct RollbackLifecycle {
        pub before: fn(step: &StepConfig),
        pub task: RollbackTaskLifecycle,
    }

    impl Default for RollbackLifecycle {
        fn default() -> Self {
            RollbackLifecycle {
                before: |_| {},
                task: Default::default(),
            }
        }
    }

    pub struct RollbackTaskLifecycle {
        pub before: fn(index: usize, rollback_task: &TaskConfig, rollback_tasks: &Vec<String>),
        pub remote_sudo: RemoteSudoLifecycle,
        pub sftp_copy: SftpCopyLifecycle,
    }

    impl Default for RollbackTaskLifecycle {
        fn default() -> Self {
            RollbackTaskLifecycle {
                before: |_, _, _| {},
                remote_sudo: Default::default(),
                sftp_copy: Default::default(),
            }
        }
    }

    pub struct RemoteSudoLifecycle {
        pub before: fn(remote_sudo: &RemoteSudoConfig),
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
        pub before: fn(sftp_copy: &SftpCopyConfig),
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
}
