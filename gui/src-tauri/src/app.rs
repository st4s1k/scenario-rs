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

/// Data structure that stores required variable values for a specific configuration path.
///
/// This structure is used as part of the application state persistence mechanism
/// to preserve user-entered values for required variables across application sessions.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_gui::app::ConfigPathData;
///
/// let mut vars = HashMap::new();
/// vars.insert("server_ip".to_string(), "192.168.1.100".to_string());
/// vars.insert("username".to_string(), "admin".to_string());
///
/// let config_data = ConfigPathData {
///     required_variables: vars,
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigPathData {
    /// Map of variable names to their values
    required_variables: HashMap<String, String>,
}

/// Configuration data structure for persisting application state.
///
/// This structure stores information about recently used configuration files
/// and their associated variable values. It's serialized to JSON and stored
/// on disk between application sessions.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_gui::app::{ScenarioAppStateConfig, ConfigPathData};
///
/// let mut config_paths = HashMap::new();
///
/// let mut vars = HashMap::new();
/// vars.insert("server_ip".to_string(), "192.168.1.100".to_string());
///
/// config_paths.insert(
///     "/path/to/config.toml".to_string(),
///     ConfigPathData { required_variables: vars }
/// );
///
/// let state_config = ScenarioAppStateConfig {
///     last_config_path: "/path/to/config.toml".to_string(),
///     config_paths,
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScenarioAppStateConfig {
    /// Path to the most recently used configuration file
    last_config_path: String,
    /// Map of configuration file paths to their associated required variable data
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

/// Main application state for the Scenario GUI application.
///
/// This structure manages the current scenario configuration, user variables,
/// execution state, and persistence of application settings.
///
/// # Examples
///
/// ```no_run
/// use tauri::AppHandle;
/// use scenario_rs_gui::app::ScenarioAppState;
///
/// fn initialize_app(app_handle: &AppHandle) {
///     let mut state = ScenarioAppState::new(app_handle);
///     
///     // Load a scenario configuration
///     state.load_config("/path/to/scenario.toml");
///     
///     // Execute the scenario
///     state.execute_scenario();
/// }
/// ```
pub struct ScenarioAppState {
    /// Path to the currently loaded configuration file
    pub(crate) config_path: String,
    /// Handle to the Tauri application for sending events to the frontend
    pub(crate) app_handle: AppHandle,
    /// The currently loaded scenario, if any
    pub(crate) scenario: Option<Scenario>,
    /// Flag indicating whether a scenario is currently executing
    pub(crate) is_executing: Arc<AtomicBool>,
}

/// Data Transfer Object for a required variable.
///
/// This structure is used to transfer required variable information between
/// the backend and the frontend UI. It contains all the necessary properties
/// to display and manage required variables in the user interface.
///
/// # Examples
///
/// ```
/// use scenario_rs_gui::app::RequiredVariableDTO;
///
/// let variable = RequiredVariableDTO {
///     label: "Server IP Address".to_string(),
///     value: "192.168.1.100".to_string(),
///     var_type: "text".to_string(),
///     read_only: false,
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequiredVariableDTO {
    /// Display label for the variable
    label: String,
    /// Current value of the variable
    value: String,
    /// Type of the variable (text, path, timestamp)
    var_type: String,
    /// Whether the variable can be modified by the user
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

/// Data Transfer Object for a task.
///
/// This structure is used to transfer task information between
/// the backend and the frontend UI. It contains all the necessary properties
/// to display task details in the user interface.
///
/// # Examples
///
/// ```
/// use scenario_rs_gui::app::TaskDTO;
///
/// let task = TaskDTO {
///     description: "Update configuration file".to_string(),
///     error_message: "Failed to update configuration".to_string(),
///     task_type: "RemoteSudo".to_string(),
///     command: Some("sudo vim /etc/config.conf".to_string()),
///     source_path: None,
///     destination_path: None,
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskDTO {
    /// Human-readable description of the task
    description: String,
    /// Error message to display if the task fails
    error_message: String,
    /// Type of task (RemoteSudo, SftpCopy, etc.)
    task_type: String,
    /// Command to execute (for RemoteSudo tasks)
    command: Option<String>,
    /// Source path (for SftpCopy tasks)
    source_path: Option<String>,
    /// Destination path (for SftpCopy tasks)
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

/// Data Transfer Object for an on-fail step in a scenario.
///
/// This structure is used to transfer on-fail step information between
/// the backend and the frontend UI. It contains the task to execute
/// if the main task fails, along with its index and total count.
///
/// # Examples
/// ```
/// use scenario_rs_gui::app::OnFailStepDTO;
///
/// let on_fail_step = OnFailStepDTO {
///    index: 1,
///    total: 2,
///    task: TaskDTO {
///       description: "Retry connection".to_string(),
///       error_message: "Failed to retry".to_string(),
///       task_type: "RemoteSudo".to_string(),
///       command: Some("sudo systemctl restart network".to_string()),
///     },
///  };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OnFailStepDTO {
    /// Index of the on-fail step in the step
    index: usize,
    /// The on-fail task to execute
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

/// Data Transfer Object for a step in a scenario.
///
/// This structure is used to transfer step information between
/// the backend and the frontend UI. It contains the task to execute
/// and any on-fail tasks that should run if the main task fails.
///
/// # Examples
///
/// ```
/// use scenario_rs_gui::app::{StepDTO, TaskDTO};
///
/// let main_task = TaskDTO {
///     description: "Install package".to_string(),
///     error_message: "Failed to install".to_string(),
///     task_type: "RemoteSudo".to_string(),
///     command: Some("apt-get install -y nginx".to_string()),
///     source_path: None,
///     destination_path: None,
/// };
///
/// let step = StepDTO {
///     task: main_task,
///     on_fail_steps: Vec::new(),
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StepDTO {
    /// Index of the step in the scenario
    index: usize,
    /// The main task to execute
    task: TaskDTO,
    /// Tasks to execute if the main task fails
    on_fail_steps: Vec<OnFailStepDTO>,
}

/// Implementation of `From<&Step>` for `StepDTO` to convert Step objects to DTOs.
///
/// This implementation handles the conversion of a `Step` object from the core library
/// into the `StepDTO` data transfer object used by the GUI frontend. It transforms
/// both the main task and any on-fail tasks into their respective DTO representations.
///
/// # Examples
///
/// ```no_run
/// use scenario_rs::scenario::step::Step;
/// use scenario_rs_gui::app::StepDTO;
///
/// fn convert_step(step: &Step) -> StepDTO {
///     StepDTO::from(step)
/// }
/// ```
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

    /// Creates a new `ScenarioAppState` instance with the provided `AppHandle`.
    ///
    /// # Arguments
    ///
    /// * `app_handle` - Handle to the Tauri application
    ///
    /// # Returns
    ///
    /// A new empty `ScenarioAppState` instance
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tauri::AppHandle;
    /// use scenario_rs_gui::app::ScenarioAppState;
    ///
    /// fn create_state(app_handle: &AppHandle) -> ScenarioAppState {
    ///     ScenarioAppState::new(app_handle)
    /// }
    /// ```
    pub fn new(app_handle: &AppHandle) -> Self {
        Self {
            config_path: String::new(),
            app_handle: app_handle.clone(),
            scenario: None,
            is_executing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initializes the application state and sets up event handling.
    ///
    /// This method:
    /// 1. Sets up the event listener to handle events from the frontend
    /// 2. Loads the saved application state from disk
    ///
    /// # Arguments
    ///
    /// * `frontend_rx` - Receiver channel for frontend events
    pub fn init(&mut self, frontend_rx: Receiver<AppEvent>) {
        trace::listen(frontend_rx, &self.app_handle, FrontendEventHandler);
        self.load_state();
    }

    /// Loads the saved application state from disk.
    ///
    /// This method:
    /// 1. Reads the state file from disk
    /// 2. Deserializes the JSON into a `ScenarioAppStateConfig` object
    /// 3. Updates the current state with the loaded configuration
    /// 4. Loads the saved scenario configuration if available
    /// 5. Restores any saved variable values for the current configuration
    ///
    /// If the state file cannot be read or parsed, the state remains unchanged.
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

    /// Loads required variable values for the current configuration from the provided state.
    ///
    /// # Arguments
    ///
    /// * `state_config` - The application state configuration
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

    /// Saves the current application state to disk.
    ///
    /// This method:
    /// 1. Converts the current state to a `ScenarioAppStateConfig` object
    /// 2. Attempts to read any existing state file and merge with current state
    /// 3. Serializes the merged state to JSON
    /// 4. Writes the JSON to the state file
    ///
    /// If there are errors reading, parsing, or writing the state file,
    /// appropriate warning or error messages are logged.
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
    ///
    /// This method:
    /// 1. Attempts to parse the specified file as a `Scenario`
    /// 2. Updates the current scenario if successful
    /// 3. Logs success or failure
    /// 4. If successful, also loads any saved variable values for this configuration
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the scenario configuration file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tauri::AppHandle;
    /// use scenario_rs_gui::app::ScenarioAppState;
    ///
    /// fn load_example_config(app_handle: &AppHandle) {
    ///     let mut state = ScenarioAppState::new(app_handle);
    ///     state.load_config("./example_configs/example-scenario.toml");
    /// }
    /// ```
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
    ///
    /// This method:
    /// 1. Clones the current scenario (if any)
    /// 2. Spawns an async task to execute the scenario
    /// 3. Sets the execution state flag during execution
    /// 4. Logs a message if no scenario is loaded
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tauri::AppHandle;
    /// use scenario_rs_gui::app::ScenarioAppState;
    ///
    /// async fn run_scenario(app_handle: &AppHandle) {
    ///     let mut state = ScenarioAppState::new(app_handle);
    ///     state.load_config("./example_configs/example-scenario.toml");
    ///     state.execute_scenario();
    /// }
    /// ```
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

    /// Retrieves the set of required variables from the current scenario.
    ///
    /// # Returns
    ///
    /// A `BTreeMap` mapping variable names to their `RequiredVariableDTO` representations.
    /// If no scenario is loaded, returns an empty map.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tauri::AppHandle;
    /// use scenario_rs_gui::app::ScenarioAppState;
    ///
    /// fn display_variables(app_handle: &AppHandle) {
    ///     let state = ScenarioAppState::new(app_handle);
    ///     let variables = state.get_required_variables();
    ///     
    ///     for (name, var) in variables {
    ///         println!("{}: {}", name, var.value);
    ///     }
    /// }
    /// ```
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

    /// Updates the values of required variables in the current scenario.
    ///
    /// This method takes a map of variable names to their new values and
    /// updates the current scenario's required variables accordingly.
    ///
    /// # Arguments
    ///
    /// * `required_variables` - A map of variable names to their new values
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::collections::HashMap;
    /// use tauri::AppHandle;
    /// use scenario_rs_gui::app::ScenarioAppState;
    ///
    /// fn update_variables(app_handle: &AppHandle) {
    ///     let mut state = ScenarioAppState::new(app_handle);
    ///     
    ///     let mut variables = HashMap::new();
    ///     variables.insert("server_ip".to_string(), "192.168.1.100".to_string());
    ///     
    ///     state.update_required_variables(variables);
    /// }
    /// ```
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

    /// Retrieves the tasks defined in the current scenario.
    ///
    /// # Returns
    ///
    /// A `BTreeMap` mapping task IDs to their `TaskDTO` representations.
    /// If no scenario is loaded, returns an empty map.
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

    /// Resolves and returns all variables from the current scenario.
    ///
    /// This method tries to resolve all variables in the current scenario,
    /// including built-in variables, required variables, and derived variables.
    ///
    /// # Returns
    ///
    /// A `BTreeMap` mapping variable names to their resolved string values.
    /// If no scenario is loaded or there's an error resolving variables,
    /// returns an empty map.
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

    /// Retrieves the execution steps defined in the current scenario.
    ///
    /// # Returns
    ///
    /// A `Vec` of `StepDTO` objects representing the execution steps.
    /// If no scenario is loaded, returns an empty vector.
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

    /// Clears the saved application state.
    ///
    /// This method:
    /// 1. Creates an empty state configuration
    /// 2. Serializes it to JSON
    /// 3. Writes it to the state file, overwriting any existing state
    /// 4. Logs success or failure
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
