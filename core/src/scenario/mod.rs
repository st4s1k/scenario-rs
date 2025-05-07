use crate::{
    config::scenario::ScenarioConfig,
    scenario::{
        credentials::Credentials, errors::ScenarioError, execute::Execute, server::Server,
        tasks::Tasks, variables::Variables,
    },
    session::Session,
    utils::HasText,
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
    ///
    /// # Examples
    ///
    /// ```
    /// use scenario_rs_core::{
    ///     config::scenario::ScenarioConfig,
    ///     scenario::Scenario
    /// };
    ///
    /// let config = ScenarioConfig::default();
    /// let scenario = Scenario::try_from(config).unwrap();
    /// let variables = scenario.variables();
    ///
    /// assert!(variables.defined().is_empty());
    /// ```
    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    /// Returns a mutable reference to the scenario's variables.
    ///
    /// # Examples
    ///
    /// ```
    /// use scenario_rs_core::{
    ///     config::scenario::ScenarioConfig,
    ///     scenario::Scenario
    /// };
    /// use std::collections::HashMap;
    ///
    /// let config = ScenarioConfig::default();
    /// let mut scenario = Scenario::try_from(config).unwrap();
    ///
    /// // Add a defined variable
    /// let mut defined = HashMap::new();
    /// defined.insert("hostname".to_string(), "example.com".to_string());
    /// scenario.variables_mut().defined_mut().extend(defined);
    ///
    /// // Verify the variable was added
    /// assert_eq!(
    ///     scenario.variables().defined().get("hostname"),
    ///     Some(&"example.com".to_string())
    /// );
    /// ```
    pub fn variables_mut(&mut self) -> &mut Variables {
        &mut self.variables
    }

    /// Returns a reference to the scenario's tasks.
    ///
    /// # Examples
    ///
    /// ```
    /// use scenario_rs_core::{
    ///     config::scenario::ScenarioConfig,
    ///     scenario::Scenario
    /// };
    ///
    /// let config = ScenarioConfig::default();
    /// let scenario = Scenario::try_from(config).unwrap();
    ///
    /// let tasks = scenario.tasks();
    /// assert!(tasks.is_empty());
    /// ```
    pub fn tasks(&self) -> &Tasks {
        &self.tasks
    }

    /// Returns a reference to the scenario's steps.
    ///
    /// # Examples
    ///
    /// ```
    /// use scenario_rs_core::{
    ///     config::scenario::ScenarioConfig,
    ///     scenario::Scenario
    /// };
    ///
    /// let config = ScenarioConfig::default();
    /// let scenario = Scenario::try_from(config).unwrap();
    ///
    /// let steps = scenario.steps();
    /// assert!(steps.is_empty());
    /// ```
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
                debug!(scenario.event = "error", scenario.error = %error);
                error
            })?;

        // Insert the username into defined variables
        let mut variables_config = config.variables.clone();
        if credentials.username.has_text() {
            variables_config
                .defined
                .insert("username".to_string(), credentials.username.clone());
        }

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
                debug!(scenario.event = "error", scenario.error = %error);
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
        debug!(scenario.event = "scenario_started");

        let session = match Session::new(&self.server, &self.credentials) {
            Ok(session) => session,
            Err(error) => {
                debug!(scenario.event = "error", scenario.error = %error);
                return;
            }
        };

        debug!(scenario.event = "session_created");

        match self.execute.steps.execute(&session, &self.variables) {
            Ok(_) => debug!(scenario.event = "scenario_completed"),
            Err(error) => debug!(scenario.event = "error", scenario.error = %error),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            credentials::CredentialsConfig, execute::ExecuteConfig, scenario::ScenarioConfig,
            server::ServerConfig, tasks::TasksConfig, variables::VariablesConfig,
        },
        scenario::Scenario,
    };
    use std::collections::HashMap;

    #[test]
    fn test_scenario_try_from_config() {
        // Given
        let config = create_test_config();

        // When
        let result = Scenario::try_from(config);

        // Then
        assert!(result.is_ok());
        let scenario = result.unwrap();
        assert_eq!(scenario.server.host, "test.example.com");
        assert_eq!(scenario.server.port, 22);
        assert_eq!(scenario.credentials.username, "testuser");
        assert_eq!(scenario.credentials.password, Some("testpass".to_string()));
        assert!(scenario.tasks.is_empty());
        assert!(scenario.steps().is_empty());
    }

    #[test]
    fn test_scenario_variables_accessor() {
        // Given
        let config = create_test_config();
        let scenario = Scenario::try_from(config).unwrap();

        // When
        let variables = scenario.variables();

        // Then
        assert_eq!(
            variables.defined().get("username"),
            Some(&"testuser".to_string())
        );
    }

    #[test]
    fn test_scenario_variables_mut_accessor() {
        // Given
        let config = create_test_config();
        let mut scenario = Scenario::try_from(config).unwrap();

        // When
        let mut new_vars = HashMap::new();
        new_vars.insert("hostname".to_string(), "newhost.example.com".to_string());
        scenario.variables_mut().defined_mut().extend(new_vars);

        // Then
        assert_eq!(
            scenario.variables().defined().get("hostname"),
            Some(&"newhost.example.com".to_string())
        );
    }

    #[test]
    fn test_scenario_tasks_accessor() {
        // Given
        let config = create_test_config();
        let scenario = Scenario::try_from(config).unwrap();

        // When
        let tasks = scenario.tasks();

        // Then
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_scenario_steps_accessor() {
        // Given
        let config = create_test_config();
        let scenario = Scenario::try_from(config).unwrap();

        // When
        let steps = scenario.steps();

        // Then
        assert!(steps.is_empty());
    }

    #[test]
    fn test_scenario_clone() {
        // Given
        let config = create_test_config();
        let original = Scenario::try_from(config).unwrap();

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(cloned.server.host, original.server.host);
        assert_eq!(cloned.credentials.username, original.credentials.username);
    }

    #[test]
    fn test_scenario_debug() {
        // Given
        let config = create_test_config();
        let scenario = Scenario::try_from(config).unwrap();

        // When
        let debug_str = format!("{:?}", scenario);

        // Then
        assert!(debug_str.contains("test.example.com"));
        assert!(debug_str.contains("testuser"));
    }

    // Test helpers
    fn create_test_config() -> ScenarioConfig {
        ScenarioConfig {
            server: ServerConfig {
                host: "test.example.com".to_string(),
                port: Some(22),
            },
            credentials: CredentialsConfig {
                username: "testuser".to_string(),
                password: Some("testpass".to_string()),
            },
            execute: ExecuteConfig::default(),
            tasks: TasksConfig::default(),
            variables: VariablesConfig::default(),
        }
    }
}
