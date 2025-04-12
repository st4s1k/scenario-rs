use crate::{
    config::scenario::ScenarioConfig,
    scenario::{
        credentials::Credentials, errors::ScenarioError, execute::Execute, server::Server,
        tasks::Tasks, variables::Variables,
    },
    session::Session,
};
use std::path::PathBuf;
use tracing::{debug, instrument};

pub mod credentials;
pub mod errors;
pub mod execute;
pub mod on_fail;
pub mod remote_sudo;
pub mod server;
pub mod sftp_copy;
pub mod step;
pub mod steps;
pub mod task;
pub mod tasks;
pub mod variables;

/// A complete deployment scenario that can be executed on a remote server.
///
/// A Scenario represents a collection of tasks, steps, and variables organized for
/// executing a specific deployment workflow. It encapsulates all necessary information
/// including server details, credentials, variables, and the sequence of operations
/// to perform.
///
/// # Example
///
/// ```
/// use scenario_rs_core::{
///     config::scenario::ScenarioConfig,
///     scenario::Scenario
/// };
/// use std::collections::HashMap;
///
/// // Create a variables config
/// let mut scenario_config = ScenarioConfig::default();
///
/// // Create a scenario from the config
/// let scenario = Scenario::try_from(scenario_config).expect("Failed to create scenario from config");
///
/// // Access scenario properties
/// assert_eq!(scenario.steps().len(), 0);
/// ```
#[derive(Clone, Debug)]
pub struct Scenario {
    pub(crate) server: Server,
    pub(crate) credentials: Credentials,
    pub(crate) execute: Execute,
    pub(crate) variables: Variables,
    pub(crate) tasks: Tasks,
}

impl Scenario {
    /// Returns a reference to the scenario's variables.
    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    /// Returns a mutable reference to the scenario's variables.
    pub fn variables_mut(&mut self) -> &mut Variables {
        &mut self.variables
    }

    /// Returns a reference to the scenario's tasks.
    pub fn tasks(&self) -> &Tasks {
        &self.tasks
    }

    /// Returns a reference to the scenario's steps.
    pub fn steps(&self) -> &steps::Steps {
        &self.execute.steps
    }
}

impl TryFrom<ScenarioConfig> for Scenario {
    type Error = ScenarioError;

    /// Attempts to create a Scenario from a configuration.
    ///
    /// This converts a configuration structure into a fully initialized Scenario
    /// ready for execution. It sets up all necessary components including server
    /// connection details, credentials, tasks, and variables.
    fn try_from(config: ScenarioConfig) -> Result<Self, Self::Error> {
        let server = Server::from(&config.server);
        let credentials = Credentials::from(&config.credentials);
        let tasks = Tasks::from(&config.tasks);
        let execute = Execute::try_from((&tasks, &config.execute))
            .map_err(ScenarioError::CannotCreateExecuteFromConfig)
            .map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?;

        // Insert the username into defined variables
        let mut variables_config = config.variables.clone();
        variables_config
            .defined
            .insert("username".to_string(), credentials.username.clone());

        let variables = Variables::from(&variables_config);

        let scenario = Scenario {
            server,
            credentials,
            execute,
            variables,
            tasks,
        };
        Ok(scenario)
    }
}

impl TryFrom<PathBuf> for Scenario {
    type Error = ScenarioError;

    /// Creates a Scenario from a configuration file path.
    ///
    /// Loads and parses the configuration file at the given path, then
    /// initializes a Scenario from it.
    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let config = ScenarioConfig::try_from(path)
            .map_err(ScenarioError::CannotCreateScenarioFromConfig)
            .map_err(|error| {
                debug!(event = "error", error = %error);
                error
            })?;
        Scenario::try_from(config)
    }
}

impl TryFrom<&str> for Scenario {
    type Error = ScenarioError;

    /// Creates a Scenario from a configuration file path string.
    ///
    /// Convenience method that converts the string to a PathBuf and loads
    /// the scenario from that path.
    fn try_from(path: &str) -> Result<Self, Self::Error> {
        let path = PathBuf::from(path);
        Scenario::try_from(path)
    }
}

impl Scenario {
    /// Executes the scenario on the remote server.
    ///
    /// This method:
    /// 1. Creates an SSH session to the target server
    /// 2. Executes all steps in the defined order
    /// 3. Logs progress via the tracing system
    ///
    /// If any step fails, execution is stopped and an error is logged.
    #[instrument(skip_all, name = "scenario")]
    pub fn execute(&self) {
        debug!(event = "scenario_started");

        let session = match Session::new(&self.server, &self.credentials) {
            Ok(session) => session,
            Err(error) => {
                debug!(event = "error", error = %error);
                return;
            }
        };

        debug!(event = "session_created");

        match self.execute.steps.execute(&session, &self.variables) {
            Ok(_) => debug!(event = "scenario_completed"),
            Err(error) => debug!(event = "error", error = %error),
        }
    }
}
