use crate::trace::{self, AppEvent, FrontendEventHandler};
use scenario_rs::{
    scenario::on_fail_step::OnFailStep,
    scenario::{
        step::Step,
        task::Task,
        variables::required::{RequiredVariable, VariableType},
        Scenario,
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
};
use tauri::AppHandle;
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

impl ScenarioAppState {
    /// Path to the file where application state is persisted
    const STATE_FILE_PATH: &'static str = "scenario-app-state.json";

    pub fn new(app_handle: &AppHandle) -> Self {
        Self {
            config_path: String::new(),
            app_handle: app_handle.clone(),
            scenario: None,
            is_executing: Arc::new(AtomicBool::new(false)),
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
            tauri::async_runtime::spawn(async move {
                is_executing.store(true, Ordering::SeqCst);
                scenario.execute();
                is_executing.store(false, Ordering::SeqCst);
            });
        } else {
            info!("No scenario loaded");
        }
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

#[cfg(test)]
mod tests {
    use crate::app::{ConfigPathData, RequiredVariableDTO, ScenarioAppStateConfig};
    use std::collections::HashMap;

    #[test]
    fn test_required_variable_dto_from_required_variable() {
        // Given
        let var_dto = RequiredVariableDTO {
            label: "Test Label".to_string(),
            value: "test_value".to_string(),
            var_type: "text".to_string(),
            read_only: true,
        };

        // When & Then
        assert_eq!(var_dto.label, "Test Label");
        assert_eq!(var_dto.value, "test_value");
        assert_eq!(var_dto.var_type, "text");
        assert_eq!(var_dto.read_only, true);
    }

    #[test]
    fn test_required_variable_dto_var_type_conversion() {
        // Given
        let string_dto = RequiredVariableDTO {
            label: "String Var".to_string(),
            value: "".to_string(),
            var_type: "text".to_string(),
            read_only: false,
        };

        let path_dto = RequiredVariableDTO {
            label: "Path Var".to_string(),
            value: "".to_string(),
            var_type: "path".to_string(),
            read_only: false,
        };

        let timestamp_dto = RequiredVariableDTO {
            label: "Timestamp Var".to_string(),
            value: "".to_string(),
            var_type: "timestamp".to_string(),
            read_only: false,
        };

        // When & Then
        assert_eq!(string_dto.var_type, "text");
        assert_eq!(path_dto.var_type, "path");
        assert_eq!(timestamp_dto.var_type, "timestamp");
    }

    #[test]
    fn test_config_path_data_stores_required_variables() {
        // Given
        let mut vars = HashMap::new();
        vars.insert("server_ip".to_string(), "192.168.1.100".to_string());
        vars.insert("username".to_string(), "admin".to_string());

        // When
        let config_data = ConfigPathData {
            required_variables: vars.clone(),
        };

        // Then
        assert_eq!(config_data.required_variables, vars);
        assert_eq!(config_data.required_variables.len(), 2);
        assert_eq!(
            config_data.required_variables.get("server_ip").unwrap(),
            "192.168.1.100"
        );
    }

    #[test]
    fn test_scenario_app_state_config_stores_last_config_path() {
        // Given
        let config_path = "/path/to/config.toml".to_string();
        let config_paths = HashMap::new();

        // When
        let state_config = ScenarioAppStateConfig {
            last_config_path: config_path.clone(),
            config_paths,
        };

        // Then
        assert_eq!(state_config.last_config_path, config_path);
        assert!(state_config.config_paths.is_empty());
    }
}
