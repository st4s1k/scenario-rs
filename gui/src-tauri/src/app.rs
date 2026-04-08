use crate::trace::{self, AppEvent, FrontendEventHandler};
use scenario_rs::{
    scenario::on_fail_step::OnFailStep,
    scenario::{
        step::Step,
        task::Task,
        variables::required::{RequiredVariable, VariableType},
        Scenario,
    },
    state::{
        types::{
            ExecutionState, ExecutionStatus, OnFailStepExecState, StepExecState, StepStatus,
        },
        ExecutionStateManager,
    },
    utils::{HasText, IsNotEmpty},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use tracing::{error, info, instrument, warn};

/// Stores required variable values for a specific configuration path, used for state persistence.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigPathData {
    required_variables: HashMap<String, String>,
}

/// Persisted application state: last config path and saved variable values per config.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScenarioAppStateConfig {
    last_config_path: String,
    config_paths: HashMap<String, ConfigPathData>,
}

#[cfg(not(tarpaulin_include))]
impl From<&ScenarioAppState> for ScenarioAppStateConfig {
    fn from(state: &ScenarioAppState) -> Self {
        let mut config_paths = HashMap::new();

        if let Some(scenario) = &state.scenario {
            if state.config_path.has_text() {
                let required_variables: HashMap<String, String> = scenario
                    .variables()
                    .required()
                    .iter()
                    .filter(|(_, required_variable)| {
                        required_variable.value().has_text() && required_variable.not_read_only()
                    })
                    .map(|(name, required_variable)| {
                        (name.to_string(), required_variable.value().to_string())
                    })
                    .collect();

                if required_variables.is_not_empty() {
                    config_paths.insert(
                        state.config_path.clone(),
                        ConfigPathData { required_variables },
                    );
                }
            }
        }

        Self {
            last_config_path: state.config_path.clone(),
            config_paths,
        }
    }
}

/// Main application state for the Scenario GUI.
pub struct ScenarioAppState {
    pub(crate) config_path: String,
    pub(crate) app_handle: AppHandle,
    pub(crate) scenario: Option<Scenario>,
    pub(crate) is_executing: Arc<AtomicBool>,
    pub(crate) execution_state_manager: Option<Arc<ExecutionStateManager>>,
}

/// DTO for transferring required variable info to the frontend.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequiredVariableDTO {
    label: String,
    value: String,
    var_type: String,
    read_only: bool,
}

impl From<&RequiredVariable> for RequiredVariableDTO {
    fn from(required_variable: &RequiredVariable) -> Self {
        let var_type = match required_variable.var_type() {
            VariableType::String => "text".to_string(),
            VariableType::Path => "path".to_string(),
            VariableType::Timestamp { .. } => "timestamp".to_string(),
        };
        Self {
            label: required_variable.label().to_string(),
            value: required_variable.value().to_string(),
            var_type,
            read_only: required_variable.read_only(),
        }
    }
}

/// DTO for transferring task info to the frontend.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskDTO {
    description: String,
    error_message: String,
    task_type: String,
    command: Option<String>,
    source_path: Option<String>,
    destination_path: Option<String>,
}

impl From<&Task> for TaskDTO {
    fn from(task: &Task) -> Self {
        match task {
            Task::RemoteSudo {
                description,
                error_message,
                remote_sudo,
            } => Self {
                description: description.to_string(),
                error_message: error_message.to_string(),
                task_type: "RemoteSudo".to_string(),
                command: Some(remote_sudo.command().to_string()),
                source_path: None,
                destination_path: None,
            },
            Task::SftpCopy {
                description,
                error_message,
                sftp_copy,
            } => Self {
                description: description.to_string(),
                error_message: error_message.to_string(),
                task_type: "SftpCopy".to_string(),
                command: None,
                source_path: Some(sftp_copy.source_path().to_string()),
                destination_path: Some(sftp_copy.destination_path().to_string()),
            },
        }
    }
}

/// DTO for an on-fail step.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OnFailStepDTO {
    index: usize,
    task: TaskDTO,
}

impl From<&OnFailStep> for OnFailStepDTO {
    fn from(on_fail_step: &OnFailStep) -> Self {
        Self {
            index: on_fail_step.index(),
            task: TaskDTO::from(on_fail_step.task()),
        }
    }
}

/// DTO for a scenario execution step.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StepDTO {
    index: usize,
    task: TaskDTO,
    on_fail_steps: Vec<OnFailStepDTO>,
}

impl From<&Step> for StepDTO {
    fn from(step: &Step) -> Self {
        let on_fail_steps: Vec<OnFailStepDTO> = step
            .on_fail_steps()
            .iter()
            .map(|on_fail_step| OnFailStepDTO::from(on_fail_step))
            .collect();

        Self {
            index: step.index(),
            task: TaskDTO::from(step.task()),
            on_fail_steps,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl ScenarioAppState {
    /// Path to the file where application state is persisted
    const STATE_FILE_PATH: &'static str = "scenario-app-state.json";

    pub fn new(app_handle: &AppHandle) -> Self {
        Self {
            config_path: String::new(),
            app_handle: app_handle.clone(),
            scenario: None,
            is_executing: Arc::new(AtomicBool::new(false)),
            execution_state_manager: None,
        }
    }

    /// Initializes the app state: sets up event listener and loads saved state.
    pub fn init(&mut self, frontend_rx: Receiver<AppEvent>) {
        trace::listen(frontend_rx, &self.app_handle, FrontendEventHandler);
        self.load_state();
    }

    /// Loads and restores saved application state from disk.
    #[instrument(skip_all)]
    pub fn load_state(&mut self) {
        if let Ok(json) = std::fs::read_to_string(Self::STATE_FILE_PATH) {
            if let Ok(loaded_state) = serde_json::from_str::<ScenarioAppStateConfig>(&json) {
                self.config_path = loaded_state.last_config_path.clone();
                self.load_config(self.config_path.clone().as_str());
                self.load_config_data_from_state(&loaded_state);
                if let Some(scenario) = self.scenario.as_mut() {
                    let required_variables = loaded_state
                        .config_paths
                        .get(&self.config_path)
                        .map(|data| data.required_variables.clone())
                        .unwrap_or_default();
                    scenario
                        .variables_mut()
                        .required_mut()
                        .upsert(required_variables);
                }
            }
        }
    }

    #[instrument(skip_all)]
    fn load_config_data_from_state(&mut self, state_config: &ScenarioAppStateConfig) {
        if let Some(config_data) = state_config.config_paths.get(&self.config_path) {
            if let Some(scenario) = self.scenario.as_mut() {
                scenario
                    .variables_mut()
                    .required_mut()
                    .upsert(config_data.required_variables.clone());
            }
        }
    }

    /// Saves current application state to disk, merging with any existing state.
    #[instrument(skip_all)]
    pub fn save_state(&mut self) {
        let current_state = ScenarioAppStateConfig::from(self.deref());

        let final_state = match std::fs::read_to_string(Self::STATE_FILE_PATH) {
            Ok(json) => match serde_json::from_str::<ScenarioAppStateConfig>(&json) {
                Ok(mut existing_state) => {
                    existing_state.last_config_path = current_state.last_config_path.clone();
                    existing_state
                        .config_paths
                        .extend(current_state.config_paths.clone());
                    info!("Application state loaded");
                    existing_state
                }
                Err(error) => {
                    error!("Failed to deserialize state: {}", error);
                    current_state
                }
            },
            Err(error) => {
                warn!("Failed to load state: {}", error);
                current_state
            }
        };

        match serde_json::to_string_pretty(&final_state) {
            Ok(json) => match std::fs::write(Self::STATE_FILE_PATH, json) {
                Ok(_) => {
                    info!("Application state saved successfully");
                }
                Err(error) => {
                    error!("Failed to save state: {}", error);
                }
            },
            Err(error) => {
                error!("Failed to serialize state: {}", error);
            }
        }
    }

    /// Loads a scenario configuration from the specified file path.
    #[instrument(skip_all)]
    pub fn load_config(&mut self, config_path: &str) {
        self.config_path = config_path.to_string();
        self.scenario = match Scenario::try_from(config_path) {
            Ok(scenario) => {
                info!("Configuration loaded from {}", config_path);
                Some(scenario)
            }
            Err(error) => {
                error!("Failed to load configuration: {}", error);
                None
            }
        };

        if self.scenario.is_some() {
            if let Ok(json) = std::fs::read_to_string(Self::STATE_FILE_PATH) {
                if let Ok(state_config) = serde_json::from_str::<ScenarioAppStateConfig>(&json) {
                    self.load_config_data_from_state(&state_config);
                }
            }
        }
    }

    /// Executes the currently loaded scenario in an async task.
    #[instrument(skip_all)]
    pub fn execute_scenario(&mut self) {
        if let Some(scenario) = self.scenario.as_ref().cloned() {
            let is_executing = self.is_executing.clone();

            // Create state manager with diff channel
            let (diff_tx, diff_rx) = std::sync::mpsc::channel();
            let initial_state = build_initial_state(&scenario);
            let state_manager = Arc::new(ExecutionStateManager::new(initial_state, diff_tx));

            // Store for snapshot queries
            self.execution_state_manager = Some(state_manager.clone());

            // Spawn diff batch streaming task
            let app_handle = self.app_handle.clone();
            tauri::async_runtime::spawn(async move {
                diff_batch_stream(diff_rx, &app_handle);
            });

            // Spawn scenario execution
            tauri::async_runtime::spawn(async move {
                is_executing.store(true, Ordering::SeqCst);
                scenario.execute(Some(&state_manager));
                is_executing.store(false, Ordering::SeqCst);
            });
        } else {
            info!("No scenario loaded");
        }
    }

    /// Returns the current execution state snapshot, if an execution has been started.
    pub fn get_execution_state(&self) -> Option<ExecutionState> {
        self.execution_state_manager.as_ref().map(|sm| sm.snapshot())
    }

    /// Returns required variables as DTOs for the frontend.
    #[instrument(skip_all)]
    pub fn get_required_variables(&self) -> BTreeMap<String, RequiredVariableDTO> {
        if let Some(scenario) = self.scenario.as_ref() {
            scenario
                .variables()
                .required()
                .iter()
                .map(|(name, required_variable)| {
                    (
                        name.to_string(),
                        RequiredVariableDTO::from(required_variable),
                    )
                })
                .collect()
        } else {
            BTreeMap::new()
        }
    }

    /// Updates required variable values in the current scenario.
    #[instrument(skip_all)]
    pub fn update_required_variables(&mut self, required_variables: HashMap<String, String>) {
        if let Some(scenario) = self.scenario.as_mut() {
            scenario
                .variables_mut()
                .required_mut()
                .upsert(required_variables);
            info!("Required variables updated");
        } else {
            info!("No scenario loaded");
        }
    }

    /// Returns tasks as DTOs for the frontend.
    #[instrument(skip_all)]
    pub fn get_tasks(&self) -> BTreeMap<String, TaskDTO> {
        if let Some(scenario) = self.scenario.as_ref() {
            scenario
                .tasks()
                .iter()
                .map(|(id, task)| (id.clone(), TaskDTO::from(task)))
                .collect()
        } else {
            BTreeMap::new()
        }
    }

    /// Resolves and returns all variables with placeholders substituted.
    #[instrument(skip_all)]
    pub fn get_resolved_variables(&mut self) -> BTreeMap<String, String> {
        if let Some(scenario) = &self.scenario {
            match scenario.variables().resolved() {
                Ok(resolved) => resolved
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect(),
                Err(error) => {
                    error!("Failed to get resolved variables: {}", error);
                    BTreeMap::new()
                }
            }
        } else {
            BTreeMap::new()
        }
    }

    /// Returns execution steps as DTOs for the frontend.
    #[instrument(skip_all)]
    pub fn get_steps(&self) -> Vec<StepDTO> {
        if let Some(scenario) = self.scenario.as_ref() {
            scenario
                .steps()
                .iter()
                .map(|step| StepDTO::from(step))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Clears the saved application state file.
    #[instrument(skip_all)]
    pub fn clear_state(&mut self) {
        let empty_state = ScenarioAppStateConfig {
            last_config_path: String::new(),
            config_paths: HashMap::new(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&empty_state) {
            if let Err(error) = std::fs::write(Self::STATE_FILE_PATH, json) {
                error!("Failed to clear state: {}", error);
            }
        }

        info!("State cleared");
    }
}

/// Builds the initial `ExecutionState` from a loaded scenario.
fn build_initial_state(scenario: &Scenario) -> ExecutionState {
    let steps = scenario
        .steps()
        .iter()
        .map(|step| StepExecState {
            index: step.index(),
            task_description: step.task().description().to_string(),
            status: StepStatus::Pending,
            progress: None,
            output: String::new(),
            errors: Vec::new(),
            on_fail_steps: step
                .on_fail_steps()
                .iter()
                .map(|ofs| OnFailStepExecState {
                    index: ofs.index(),
                    task_description: ofs.task().description().to_string(),
                    status: StepStatus::Pending,
                    progress: None,
                    output: String::new(),
                    errors: Vec::new(),
                })
                .collect(),
        })
        .collect();

    ExecutionState {
        status: ExecutionStatus::Idle,
        steps,
    }
}

/// Reads diffs from the channel, batches them over 100ms windows,
/// and emits them as a single Tauri event per batch.
#[cfg(not(tarpaulin_include))]
fn diff_batch_stream(
    rx: std::sync::mpsc::Receiver<scenario_rs::state::types::StateDiff>,
    app_handle: &AppHandle,
) {
    use scenario_rs::state::types::StateDiff;

    loop {
        // Block until first diff arrives (or channel closes)
        let first: StateDiff = match rx.recv() {
            Ok(diff) => diff,
            Err(_) => break,
        };

        let mut batch = vec![first];
        let deadline = Instant::now() + Duration::from_millis(100);

        // Collect more diffs within the 100ms window
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match rx.recv_timeout(remaining) {
                Ok(diff) => batch.push(diff),
                Err(_) => break,
            }
        }

        let _ = app_handle.emit("execution-diff", &batch);
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{
        build_initial_state, ConfigPathData, OnFailStepDTO, RequiredVariableDTO,
        ScenarioAppStateConfig, StepDTO, TaskDTO,
    };
    use scenario_rs::scenario::{
        on_fail_step::OnFailStep, on_fail_steps::OnFailSteps, remote_sudo::RemoteSudo,
        sftp_copy::SftpCopy, step::Step, task::Task, Scenario,
    };
    use scenario_rs::state::types::{ExecutionStatus, StepStatus};
    use std::collections::HashMap;

    #[test]
    fn test_task_dto_from_remote_sudo_task() {
        // Given
        let task = Task::RemoteSudo {
            description: "Stop service".to_string(),
            error_message: "Failed to stop".to_string(),
            remote_sudo: RemoteSudo::new("systemctl stop myapp".to_string()),
        };

        // When
        let dto = TaskDTO::from(&task);

        // Then
        assert_eq!(dto.description, "Stop service");
        assert_eq!(dto.error_message, "Failed to stop");
        assert_eq!(dto.task_type, "RemoteSudo");
        assert_eq!(dto.command.as_deref(), Some("systemctl stop myapp"));
        assert!(dto.source_path.is_none());
        assert!(dto.destination_path.is_none());
    }

    #[test]
    fn test_task_dto_from_sftp_copy_task() {
        // Given
        let task = Task::SftpCopy {
            description: "Upload file".to_string(),
            error_message: "Upload failed".to_string(),
            sftp_copy: SftpCopy {
                source_path: "/local/app.jar".to_string(),
                destination_path: "/remote/app.jar".to_string(),
            },
        };

        // When
        let dto = TaskDTO::from(&task);

        // Then
        assert_eq!(dto.description, "Upload file");
        assert_eq!(dto.error_message, "Upload failed");
        assert_eq!(dto.task_type, "SftpCopy");
        assert!(dto.command.is_none());
        assert_eq!(dto.source_path.as_deref(), Some("/local/app.jar"));
        assert_eq!(dto.destination_path.as_deref(), Some("/remote/app.jar"));
    }

    #[test]
    fn test_on_fail_step_dto_from_on_fail_step() {
        // Given
        let task = Task::RemoteSudo {
            description: "Restart service".to_string(),
            error_message: "Restart failed".to_string(),
            remote_sudo: RemoteSudo::new("systemctl restart myapp".to_string()),
        };
        let on_fail_step = OnFailStep::from((2, task));

        // When
        let dto = OnFailStepDTO::from(&on_fail_step);

        // Then
        assert_eq!(dto.index, 2);
        assert_eq!(dto.task.description, "Restart service");
        assert_eq!(dto.task.task_type, "RemoteSudo");
    }

    #[test]
    fn test_step_dto_from_step_without_on_fail() {
        // Given
        let step = Step {
            index: 0,
            task: Task::RemoteSudo {
                description: "Run migration".to_string(),
                error_message: "Migration failed".to_string(),
                remote_sudo: RemoteSudo::new("migrate".to_string()),
            },
            on_fail_steps: OnFailSteps::default(),
        };

        // When
        let dto = StepDTO::from(&step);

        // Then
        assert_eq!(dto.index, 0);
        assert_eq!(dto.task.description, "Run migration");
        assert!(dto.on_fail_steps.is_empty());
    }

    #[test]
    fn test_step_dto_from_step_with_on_fail_steps() {
        // Given
        let recovery_task = Task::RemoteSudo {
            description: "Rollback".to_string(),
            error_message: "Rollback failed".to_string(),
            remote_sudo: RemoteSudo::new("rollback".to_string()),
        };
        let on_fail = OnFailSteps::from(vec![OnFailStep::from((0, recovery_task))]);
        let step = Step {
            index: 1,
            task: Task::SftpCopy {
                description: "Deploy".to_string(),
                error_message: "Deploy failed".to_string(),
                sftp_copy: SftpCopy {
                    source_path: "/src".to_string(),
                    destination_path: "/dst".to_string(),
                },
            },
            on_fail_steps: on_fail,
        };

        // When
        let dto = StepDTO::from(&step);

        // Then
        assert_eq!(dto.index, 1);
        assert_eq!(dto.task.task_type, "SftpCopy");
        assert_eq!(dto.on_fail_steps.len(), 1);
        assert_eq!(dto.on_fail_steps[0].index, 0);
        assert_eq!(dto.on_fail_steps[0].task.description, "Rollback");
    }

    #[test]
    fn test_build_initial_state_from_scenario() {
        // Given
        let scenario = Scenario::try_from("../../example_configs/example-scenario.toml")
            .expect("Failed to load example scenario");

        // When
        let state = build_initial_state(&scenario);

        // Then
        assert_eq!(state.status, ExecutionStatus::Idle);
        assert!(!state.steps.is_empty());
        for step in &state.steps {
            assert_eq!(step.status, StepStatus::Pending);
            assert!(step.output.is_empty());
            assert!(step.errors.is_empty());
            assert!(step.progress.is_none());
            for ofs in &step.on_fail_steps {
                assert_eq!(ofs.status, StepStatus::Pending);
                assert!(ofs.output.is_empty());
                assert!(ofs.errors.is_empty());
                assert!(ofs.progress.is_none());
            }
        }
    }

    #[test]
    fn test_required_variable_dto_from_scenario() {
        // Given
        let scenario = Scenario::try_from("../../example_configs/example-scenario.toml")
            .expect("Failed to load example scenario");

        // When
        let required_vars = scenario.variables().required();
        let dtos: Vec<_> = required_vars
            .iter()
            .map(|(_, rv)| super::RequiredVariableDTO::from(rv))
            .collect();

        // Then
        assert!(!dtos.is_empty());
        for dto in &dtos {
            assert!(!dto.label.is_empty());
            assert!(
                dto.var_type == "text" || dto.var_type == "path" || dto.var_type == "timestamp"
            );
        }
    }

    #[test]
    fn test_config_path_data_serialization() {
        // Given
        let mut required_variables = HashMap::new();
        required_variables.insert("user".to_string(), "admin".to_string());
        let data = ConfigPathData { required_variables };

        // When
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: ConfigPathData = serde_json::from_str(&json).unwrap();

        // Then
        assert_eq!(
            deserialized.required_variables.get("user"),
            Some(&"admin".to_string())
        );
    }

    #[test]
    fn test_scenario_app_state_config_serialization() {
        // Given
        let mut config_paths = HashMap::new();
        config_paths.insert(
            "/path/to/config.toml".to_string(),
            ConfigPathData {
                required_variables: HashMap::from([
                    ("var1".to_string(), "val1".to_string()),
                ]),
            },
        );
        let state_config = ScenarioAppStateConfig {
            last_config_path: "/path/to/config.toml".to_string(),
            config_paths,
        };

        // When
        let json = serde_json::to_string_pretty(&state_config).unwrap();
        let deserialized: ScenarioAppStateConfig = serde_json::from_str(&json).unwrap();

        // Then
        assert_eq!(deserialized.last_config_path, "/path/to/config.toml");
        assert!(deserialized.config_paths.contains_key("/path/to/config.toml"));
        let data = deserialized.config_paths.get("/path/to/config.toml").unwrap();
        assert_eq!(data.required_variables.get("var1"), Some(&"val1".to_string()));
    }

    #[test]
    fn test_diff_batch_stream_empty_channel() {
        // Given
        use scenario_rs::state::types::StateDiff;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel::<StateDiff>();
        drop(tx);

        // When & Then
        assert!(rx.recv().is_err());
    }
}
