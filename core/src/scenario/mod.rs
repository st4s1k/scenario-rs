use crate::{
    config::scenario::ScenarioConfig,
    scenario::{
        credentials::Credentials, errors::ScenarioError, execute::Execute, server::Server,
        tasks::Tasks, variables::Variables,
    },
    session::{Session, SshError},
    state::ExecutionStateManager,
    state::types::ExecutionStatus,
    trace::ScenarioEvent,
    utils::HasText,
};
use std::path::PathBuf;
use tracing::{debug, instrument};

#[cfg(not(tarpaulin_include))]
fn log_scenario_event(event: ScenarioEvent) {
    debug!(scenario.event = event.as_str());
}

#[cfg(not(tarpaulin_include))]
fn log_scenario_error(error: &str) {
    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = error);
    debug!(scenario.event = ScenarioEvent::ScenarioFailed.as_str());
}

enum ExecutionOutcome {
    Completed,
    SessionFailed(String),
    StepsFailed(String),
}

pub mod credentials;
pub mod errors;
pub mod execute;
pub mod on_fail_step;
pub mod on_fail_steps;
pub mod remote_sudo;
pub mod server;
pub mod sftp_copy;
pub mod step;
pub mod steps;
pub mod task;
pub mod tasks;
pub mod variables;

/// A complete deployment scenario ready for execution on a remote server.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub(crate) server: Server,
    pub(crate) credentials: Credentials,
    pub(crate) execute: Execute,
    pub(crate) variables: Variables,
    pub(crate) tasks: Tasks,
}

impl Scenario {
    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    pub fn variables_mut(&mut self) -> &mut Variables {
        &mut self.variables
    }

    pub fn tasks(&self) -> &Tasks {
        &self.tasks
    }

    pub fn steps(&self) -> &steps::Steps {
        &self.execute.steps
    }
}

impl TryFrom<ScenarioConfig> for Scenario {
    type Error = ScenarioError;

    fn try_from(config: ScenarioConfig) -> Result<Self, Self::Error> {
        let server = Server::from(&config.server);
        let credentials = Credentials::from(&config.credentials);
        let tasks = Tasks::from(&config.tasks);
        let execute = Execute::try_from((&tasks, &config.execute))
            .map_err(ScenarioError::CannotCreateExecuteFromConfig)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
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

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let config = ScenarioConfig::try_from(path)
            .map_err(ScenarioError::CannotCreateScenarioFromConfig)
            .map_err(|error| {
                debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                error
            })?;
        Scenario::try_from(config)
    }
}

impl TryFrom<&str> for Scenario {
    type Error = ScenarioError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        let path = PathBuf::from(path);
        Scenario::try_from(path)
    }
}

impl Scenario {
    /// Executes the scenario: creates SSH session, runs steps, logs progress.
    ///
    /// Pass an `ExecutionStateManager` to enable structured progress tracking.
    /// Pass `None` for tracing-only output (e.g. CLI usage).
    /// When `debug_mode` is true, uses a mock session instead of real SSH.
    #[instrument(skip_all, name = "scenario")]
    #[cfg(not(tarpaulin_include))]
    pub fn execute(&self, state_manager: Option<&ExecutionStateManager>, debug_mode: bool) {
        log_scenario_event(ScenarioEvent::ScenarioStarted);
        let session_result = Session::new(&self.server, &self.credentials, debug_mode);
        match self.execute_with_session(session_result, state_manager) {
            ExecutionOutcome::Completed => {
                log_scenario_event(ScenarioEvent::ScenarioCompleted);
            }
            ExecutionOutcome::SessionFailed(error) => {
                log_scenario_error(&error);
            }
            ExecutionOutcome::StepsFailed(error) => {
                log_scenario_error(&error);
            }
        }
    }

    fn execute_with_session(
        &self,
        session_result: Result<Session, SshError>,
        state_manager: Option<&ExecutionStateManager>,
    ) -> ExecutionOutcome {
        if let Some(sm) = state_manager {
            sm.update_execution_status(ExecutionStatus::Running);
        }

        let session = match session_result {
            Ok(session) => session,
            Err(error) => {
                let msg = error.to_string();
                if let Some(sm) = state_manager {
                    sm.update_execution_status(ExecutionStatus::Failed {
                        error: msg.clone(),
                    });
                }
                return ExecutionOutcome::SessionFailed(msg);
            }
        };

        log_scenario_event(ScenarioEvent::SessionCreated);

        match self
            .execute
            .steps
            .execute(&session, &self.variables, state_manager)
        {
            Ok(_) => {
                if let Some(sm) = state_manager {
                    sm.update_execution_status(ExecutionStatus::Completed);
                }
                ExecutionOutcome::Completed
            }
            Err(error) => {
                let msg = error.to_string();
                if let Some(sm) = state_manager {
                    sm.update_execution_status(ExecutionStatus::Failed {
                        error: msg.clone(),
                    });
                }
                ExecutionOutcome::StepsFailed(msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            credentials::CredentialsConfig, execute::ExecuteConfig, scenario::ScenarioConfig,
            server::ServerConfig, step::StepConfig, tasks::TasksConfig, variables::VariablesConfig,
        },
        scenario::Scenario,
        session::{Session, SshError},
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::TRACE)
            .try_init();
    }

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

    fn create_test_config() -> ScenarioConfig {
        ScenarioConfig {
            server: ServerConfig {
                host: "test.example.com".to_string(),
                port: Some(22),
            },
            credentials: CredentialsConfig {
                username: "testuser".to_string(),
                password: Some("testpass".to_string()),
                private_key: None,
            },
            execute: ExecuteConfig::default(),
            tasks: TasksConfig::default(),
            variables: VariablesConfig::default(),
        }
    }

    #[test]
    fn test_scenario_try_from_pathbuf_with_valid_config() {
        // Given
        let path = PathBuf::from("../example_configs/example-scenario.toml");

        // When
        let result = Scenario::try_from(path);

        // Then
        assert!(result.is_ok());
        let scenario = result.unwrap();
        assert_eq!(scenario.server.host, "localhost");
        assert_eq!(scenario.server.port, 22);
    }

    #[test]
    fn test_scenario_try_from_pathbuf_with_nonexistent_file() {
        // Given
        let path = PathBuf::from("nonexistent/config.toml");

        // When
        let result = Scenario::try_from(path);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_scenario_try_from_str_with_valid_config() {
        // Given
        let path = "../example_configs/example-scenario.toml";

        // When
        let result = Scenario::try_from(path);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_scenario_try_from_str_with_nonexistent_file() {
        // Given
        let path = "nonexistent/config.toml";

        // When
        let result = Scenario::try_from(path);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_scenario_try_from_config_with_invalid_execute() {
        init_tracing();

        // Given
        let config = ScenarioConfig {
            server: ServerConfig {
                host: "test.example.com".to_string(),
                port: Some(22),
            },
            credentials: CredentialsConfig {
                username: "testuser".to_string(),
                password: Some("testpass".to_string()),
                private_key: None,
            },
            execute: ExecuteConfig {
                steps: vec![StepConfig {
                    task: "nonexistent_task".to_string(),
                    on_fail: None,
                }]
                .into(),
            },
            tasks: TasksConfig::default(),
            variables: VariablesConfig::default(),
        };

        // When
        let result = Scenario::try_from(config);

        // Then
        assert!(result.is_err());
    }

    #[test]
    #[cfg(not(tarpaulin_include))]
    fn test_scenario_execute_completes_successfully() {
        init_tracing();
        // Given
        let scenario =
            Scenario::try_from("../example_configs/example-scenario.toml").unwrap();

        // When
        scenario.execute(None, true);

        // Then
    }

    #[test]
    #[cfg(not(tarpaulin_include))]
    fn test_scenario_execute_with_state_manager() {
        // Given
        use crate::state::{
            types::{ExecutionState, ExecutionStatus, StepStatus},
            ExecutionStateManager,
        };
        use std::sync::mpsc;

        let scenario =
            Scenario::try_from("../example_configs/example-scenario.toml").unwrap();

        let initial_state = ExecutionState {
            status: ExecutionStatus::Idle,
            steps: scenario
                .steps()
                .iter()
                .map(|step| crate::state::types::StepExecState {
                    index: step.index(),
                    task_description: step.task().description().to_string(),
                    status: StepStatus::Pending,
                    progress: None,
                    output: String::new(),
                    errors: Vec::new(),
                    on_fail_steps: Vec::new(),
                })
                .collect(),
        };

        let (tx, _rx) = mpsc::channel();
        let sm = ExecutionStateManager::new(initial_state, tx);

        // When
        scenario.execute(Some(&sm), true);

        // Then
        let snapshot = sm.snapshot();
        assert!(
            matches!(snapshot.status, ExecutionStatus::Completed | ExecutionStatus::Failed { .. }),
        );
    }

    #[test]
    fn test_scenario_execute_with_session_failure() {
        init_tracing();
        // Given
        use crate::state::{
            types::{ExecutionState, ExecutionStatus},
            ExecutionStateManager,
        };
        use std::sync::mpsc;

        let scenario =
            Scenario::try_from("../example_configs/example-scenario.toml").unwrap();

        let initial_state = ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![],
        };
        let (tx, _rx) = mpsc::channel();
        let sm = ExecutionStateManager::new(initial_state, tx);

        let session_error = SshError::new("test session error");

        // When
        let outcome = scenario.execute_with_session(Err(session_error), Some(&sm));

        // Then
        assert!(matches!(outcome, super::ExecutionOutcome::SessionFailed(_)));
        let snapshot = sm.snapshot();
        assert!(matches!(snapshot.status, ExecutionStatus::Failed { .. }));
    }

    #[test]
    fn test_scenario_execute_with_session_failure_no_state_manager() {
        init_tracing();
        // Given
        let scenario =
            Scenario::try_from("../example_configs/example-scenario.toml").unwrap();

        let session_error = SshError::new("test session error");

        // When
        let outcome = scenario.execute_with_session(Err(session_error), None);

        // Then
        assert!(matches!(outcome, super::ExecutionOutcome::SessionFailed(_)));
    }

    #[test]
    fn test_scenario_execute_with_steps_failure() {
        init_tracing();
        // Given
        use crate::{
            session::SessionType,
            utils::{ArcMutex, Wrap},
            state::{
                types::{ExecutionState, ExecutionStatus, StepExecState, StepStatus},
                ExecutionStateManager,
            },
        };
        use std::sync::mpsc;

        struct FailChannel;
        impl crate::session::Channel for FailChannel {
            fn exec(&mut self, _command: &str) -> Result<(), SshError> {
                Err(SshError::new("exec error"))
            }
            fn read_to_string(&mut self, _: &mut String) -> Result<usize, SshError> {
                Ok(0)
            }
            fn exit_status(&self) -> Result<i32, SshError> {
                Ok(0)
            }
        }

        struct TestSftp;
        impl crate::session::Sftp for TestSftp {
            fn create(
                &self,
                _path: &std::path::Path,
            ) -> Result<Box<dyn crate::session::Write>, SshError> {
                struct TestWrite;
                impl crate::session::Write for TestWrite {
                    fn write_all(&mut self, _buf: &[u8]) -> Result<(), SshError> {
                        Ok(())
                    }
                }
                Ok(Box::new(TestWrite))
            }
        }

        let scenario =
            Scenario::try_from("../example_configs/example-scenario.toml").unwrap();

        let fail_session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(FailChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };

        let initial_state = ExecutionState {
            status: ExecutionStatus::Idle,
            steps: scenario
                .steps()
                .iter()
                .map(|step| StepExecState {
                    index: step.index(),
                    task_description: step.task().description().to_string(),
                    status: StepStatus::Pending,
                    progress: None,
                    output: String::new(),
                    errors: Vec::new(),
                    on_fail_steps: Vec::new(),
                })
                .collect(),
        };
        let (tx, _rx) = mpsc::channel();
        let sm = ExecutionStateManager::new(initial_state, tx);

        // When
        let outcome = scenario.execute_with_session(Ok(fail_session), Some(&sm));

        // Then
        assert!(matches!(outcome, super::ExecutionOutcome::StepsFailed(_)));
        let snapshot = sm.snapshot();
        assert!(matches!(snapshot.status, ExecutionStatus::Failed { .. }));
    }

    #[test]
    fn test_scenario_execute_with_session_success() {
        init_tracing();
        use crate::{
            session::SessionType,
            utils::{ArcMutex, Wrap},
            state::{
                types::{ExecutionState, ExecutionStatus},
                ExecutionStateManager,
            },
        };
        use std::sync::mpsc;

        struct OkChannel;
        impl crate::session::Channel for OkChannel {
            fn exec(&mut self, _command: &str) -> Result<(), SshError> {
                Ok(())
            }
            fn read_to_string(&mut self, _: &mut String) -> Result<usize, SshError> {
                Ok(0)
            }
            fn exit_status(&self) -> Result<i32, SshError> {
                Ok(0)
            }
        }

        struct TestSftp;
        impl crate::session::Sftp for TestSftp {
            fn create(
                &self,
                _path: &std::path::Path,
            ) -> Result<Box<dyn crate::session::Write>, SshError> {
                struct TestWrite;
                impl crate::session::Write for TestWrite {
                    fn write_all(&mut self, _buf: &[u8]) -> Result<(), SshError> {
                        Ok(())
                    }
                }
                Ok(Box::new(TestWrite))
            }
        }

        // Given
        let config = create_test_config();
        let scenario = Scenario::try_from(config).unwrap();

        let session = Session {
            inner: SessionType::Test {
                channel: ArcMutex::wrap(OkChannel),
                sftp: ArcMutex::wrap(TestSftp),
            },
        };

        let initial_state = ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![],
        };
        let (tx, _rx) = mpsc::channel();
        let sm = ExecutionStateManager::new(initial_state, tx);

        // When
        let outcome = scenario.execute_with_session(Ok(session), Some(&sm));

        // Then
        assert!(matches!(outcome, super::ExecutionOutcome::Completed));
        let snapshot = sm.snapshot();
        assert!(matches!(snapshot.status, ExecutionStatus::Completed));
    }
}
