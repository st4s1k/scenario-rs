use crate::trace::{self, AppEvent, FrontendEventHandler};
use scenario_rs::{
    config::scenario::{PartialScenarioConfig, ScenarioConfig},
    config::task::{RemoteSudoTaskConfig, SftpCopyTaskConfig},
    config::tasks::TasksConfig,
    config::variables::PartialVariablesConfig,
    config::variables::defined::DefinedVariablesConfig,
    scenario::on_fail_step::OnFailStep,
    scenario::{
        step::Step,
        task::Task,
        variables::required::RequiredVariable,
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
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
    time::{Duration, Instant},
};

/// Drop guard that resets `is_executing` to `false` when the execution task ends.
struct ExecutingGuard(Arc<AtomicBool>);

impl Drop for ExecutingGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
use tauri::{AppHandle, Emitter};
use tracing::{error, info, instrument, warn};

/// Port for persisting/loading application state.
/// Production uses `FileStateStorage`; tests use `InMemoryStateStorage`.
pub trait StateStorage: Send {
    fn read(&self) -> std::io::Result<String>;
    fn write(&self, content: &str) -> std::io::Result<()>;
}

/// Production implementation — reads/writes a JSON file on disk.
#[cfg(not(tarpaulin_include))]
pub struct FileStateStorage {
    path: String,
}

#[cfg(not(tarpaulin_include))]
impl FileStateStorage {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

#[cfg(not(tarpaulin_include))]
impl StateStorage for FileStateStorage {
    fn read(&self) -> std::io::Result<String> {
        std::fs::read_to_string(&self.path)
    }
    fn write(&self, content: &str) -> std::io::Result<()> {
        std::fs::write(&self.path, content)
    }
}

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
        ScenarioAppStateConfig::from(&state.core)
    }
}

impl From<&ScenarioAppStateCore> for ScenarioAppStateConfig {
    fn from(core: &ScenarioAppStateCore) -> Self {
        let mut config_paths = HashMap::new();

        if let Some(scenario) = &core.scenario {
            if core.config_path.has_text() {
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
                        core.config_path.clone(),
                        ConfigPathData { required_variables },
                    );
                }
            }
        }

        Self {
            last_config_path: core.config_path.clone(),
            config_paths,
        }
    }
}

/// Core application state without Tauri-specific dependencies.
/// All pure logic methods live here so they can be unit-tested directly.
pub struct ScenarioAppStateCore {
    pub(crate) config_path: String,
    pub(crate) scenario: Option<Scenario>,
    pub(crate) leaf_config: Option<PartialScenarioConfig>,
    pub(crate) is_executing: Arc<AtomicBool>,
    pub(crate) dry_run: bool,
    pub(crate) execution_state_manager: Option<Arc<ExecutionStateManager>>,
    pub(crate) state_storage: Box<dyn StateStorage>,
}

/// Main application state for the Scenario GUI.
pub struct ScenarioAppState {
    pub(crate) core: ScenarioAppStateCore,
    pub(crate) app_handle: AppHandle,
}

#[cfg(not(tarpaulin_include))]
impl Deref for ScenarioAppState {
    type Target = ScenarioAppStateCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

#[cfg(not(tarpaulin_include))]
impl std::ops::DerefMut for ScenarioAppState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}

/// DTO for transferring required variable info to the frontend.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequiredVariableDTO {
    label: String,
    value: String,
    read_only: bool,
    file_picker: bool,
}

impl From<&RequiredVariable> for RequiredVariableDTO {
    fn from(required_variable: &RequiredVariable) -> Self {
        Self {
            label: required_variable.label().to_string(),
            value: required_variable.value().to_string(),
            read_only: required_variable.read_only(),
            file_picker: required_variable.file_picker(),
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
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ConfigDiff {
    pub modified_tasks: Vec<String>,
    pub modified_variables: Vec<String>,
}

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

impl ScenarioAppStateCore {
    /// Path to the file where application state is persisted
    const DEFAULT_STATE_FILE_PATH: &'static str = "scenario-app-state.json";

    /// Loads and restores saved application state from disk.
    #[instrument(skip_all)]
    pub fn load_state(&mut self) {
        if let Ok(json) = self.state_storage.read() {
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
        let current_state = ScenarioAppStateConfig::from(&*self);

        let final_state = match self.state_storage.read() {
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

        let json = serde_json::to_string_pretty(&final_state)
            .expect("Failed to serialize application state");
        match self.state_storage.write(&json) {
            Ok(_) => {
                info!("Application state saved successfully");
            }
            Err(error) => {
                error!("Failed to save state: {}", error);
            }
        }
    }

    /// Loads a scenario configuration from the specified file path.
    #[instrument(skip_all)]
    pub fn load_config(&mut self, config_path: &str) {
        self.config_path = config_path.to_string();
        match ScenarioConfig::load_with_leaf(PathBuf::from(config_path)) {
            Ok((config, leaf)) => {
                self.leaf_config = Some(leaf);
                match Scenario::try_from(config) {
                    Ok(scenario) => {
                        info!("Configuration loaded from {}", config_path);
                        self.scenario = Some(scenario);
                    }
                    Err(error) => {
                        error!("Failed to build scenario: {}", error);
                        self.scenario = None;
                    }
                }
            }
            Err(error) => {
                error!("Failed to load configuration: {}", error);
                self.scenario = None;
                self.leaf_config = None;
            }
        };

        if self.scenario.is_some() {
            if let Ok(json) = self.state_storage.read() {
                if let Ok(state_config) = serde_json::from_str::<ScenarioAppStateConfig>(&json) {
                    self.load_config_data_from_state(&state_config);
                }
            }
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

        let json = serde_json::to_string_pretty(&empty_state)
            .expect("Failed to serialize empty state");
        if let Err(error) = self.state_storage.write(&json) {
            error!("Failed to clear state: {}", error);
        }

        info!("State cleared");
    }

    /// Updates a task in both the runtime scenario and the leaf config.
    #[instrument(skip_all)]
    pub fn update_task(&mut self, task_name: String, task_dto: TaskDTO) -> Result<(), String> {
        let scenario = self.scenario.as_mut().ok_or("No scenario loaded")?;
        let leaf = self.leaf_config.as_mut().ok_or("No leaf config loaded")?;

        // Update the runtime scenario
        match task_dto.task_type.as_str() {
            "RemoteSudo" => {
                let command = task_dto.command.clone().unwrap_or_default();
                let task = Task::RemoteSudo {
                    description: task_dto.description.clone(),
                    error_message: task_dto.error_message.clone(),
                    remote_sudo: scenario_rs::scenario::remote_sudo::RemoteSudo::new(command.clone()),
                };
                scenario.tasks_mut().insert(task_name.clone(), task);

                // Update leaf config
                let remote_sudo = leaf.tasks.get_or_insert_with(Default::default)
                    .remote_sudo.get_or_insert_with(Default::default);
                remote_sudo.insert(task_name, RemoteSudoTaskConfig {
                    command,
                    description: Some(task_dto.description).filter(|s| !s.is_empty()),
                    error_message: Some(task_dto.error_message).filter(|s| !s.is_empty()),
                });
            }
            "SftpCopy" => {
                let source = task_dto.source_path.clone().unwrap_or_default();
                let destination = task_dto.destination_path.clone().unwrap_or_default();
                let task = Task::SftpCopy {
                    description: task_dto.description.clone(),
                    error_message: task_dto.error_message.clone(),
                    sftp_copy: scenario_rs::scenario::sftp_copy::SftpCopy {
                        source_path: source.clone(),
                        destination_path: destination.clone(),
                    },
                };
                scenario.tasks_mut().insert(task_name.clone(), task);

                // Update leaf config
                let sftp_copy = leaf.tasks.get_or_insert_with(Default::default)
                    .sftp_copy.get_or_insert_with(Default::default);
                sftp_copy.insert(task_name, SftpCopyTaskConfig {
                    source,
                    destination,
                    description: Some(task_dto.description).filter(|s| !s.is_empty()),
                    error_message: Some(task_dto.error_message).filter(|s| !s.is_empty()),
                });
            }
            _ => return Err(format!("Unknown task type: {}", task_dto.task_type)),
        }

        info!("Task updated");
        Ok(())
    }

    /// Updates a defined variable in both the runtime scenario and the leaf config.
    #[instrument(skip_all)]
    pub fn update_defined_variable(&mut self, name: String, value: String) -> Result<(), String> {
        let scenario = self.scenario.as_mut().ok_or("No scenario loaded")?;
        let leaf = self.leaf_config.as_mut().ok_or("No leaf config loaded")?;

        // Update runtime
        scenario.variables_mut().defined_mut().insert(name.clone(), value.clone());

        // Update leaf config
        let variables = leaf.variables.get_or_insert_with(Default::default);
        let defined = variables.defined.get_or_insert_with(Default::default);
        defined.insert(name, value);

        info!("Defined variable updated");
        Ok(())
    }

    /// Computes a per-item diff between the in-memory leaf config and the on-disk file.
    /// Returns the names of modified (or added/removed) tasks and defined variables.
    pub fn get_config_diff(&self) -> ConfigDiff {
        let mut diff = ConfigDiff::default();

        let Some(current) = &self.leaf_config else { return diff };
        if self.config_path.is_empty() { return diff };

        let Ok(file_contents) = std::fs::read_to_string(&self.config_path) else { return diff };
        let Ok(disk) = toml::from_str::<PartialScenarioConfig>(&file_contents) else { return diff };

        let empty_tasks = TasksConfig::default();
        let current_tasks = current.tasks.as_ref().unwrap_or(&empty_tasks);
        let disk_tasks = disk.tasks.as_ref().unwrap_or(&empty_tasks);

        let empty_remote: HashMap<String, RemoteSudoTaskConfig> = HashMap::new();
        let current_remote = current_tasks.remote_sudo.as_ref().unwrap_or(&empty_remote);
        let disk_remote = disk_tasks.remote_sudo.as_ref().unwrap_or(&empty_remote);
        for (name, task) in current_remote {
            if disk_remote.get(name) != Some(task) {
                diff.modified_tasks.push(name.clone());
            }
        }
        for name in disk_remote.keys() {
            if !current_remote.contains_key(name) && !diff.modified_tasks.contains(name) {
                diff.modified_tasks.push(name.clone());
            }
        }

        let empty_sftp: HashMap<String, SftpCopyTaskConfig> = HashMap::new();
        let current_sftp = current_tasks.sftp_copy.as_ref().unwrap_or(&empty_sftp);
        let disk_sftp = disk_tasks.sftp_copy.as_ref().unwrap_or(&empty_sftp);
        for (name, task) in current_sftp {
            if disk_sftp.get(name) != Some(task) {
                diff.modified_tasks.push(name.clone());
            }
        }
        for name in disk_sftp.keys() {
            if !current_sftp.contains_key(name) && !diff.modified_tasks.contains(name) {
                diff.modified_tasks.push(name.clone());
            }
        }

        let empty_vars = PartialVariablesConfig::default();
        let current_vars = current.variables.as_ref().unwrap_or(&empty_vars);
        let disk_vars = disk.variables.as_ref().unwrap_or(&empty_vars);

        let empty_defined = DefinedVariablesConfig::default();
        let current_defined = current_vars.defined.as_ref().unwrap_or(&empty_defined);
        let disk_defined = disk_vars.defined.as_ref().unwrap_or(&empty_defined);
        for (name, value) in current_defined.iter() {
            if disk_defined.get(name) != Some(value) {
                diff.modified_variables.push(name.clone());
            }
        }
        for name in disk_defined.keys() {
            if !current_defined.contains_key(name) && !diff.modified_variables.contains(name) {
                diff.modified_variables.push(name.clone());
            }
        }

        diff
    }

    /// Discards in-memory config changes by reloading from the config file on disk.
    pub fn discard_config_changes(&mut self) {
        if !self.config_path.is_empty() {
            let path = self.config_path.clone();
            self.load_config(&path);
        }
    }

    /// Serializes the leaf config to TOML and writes it to the config file.
    #[instrument(skip_all)]
    pub fn save_config(&self) -> Result<(), String> {
        let leaf = self.leaf_config.as_ref().ok_or("No leaf config loaded")?;

        if self.config_path.is_empty() {
            return Err("No config path set".to_string());
        }

        let toml_string = toml::to_string_pretty(leaf)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(&self.config_path, toml_string)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        info!("Config saved to {}", self.config_path);
        Ok(())
    }

    /// Reads the on-disk config, returning a default if the path is empty or the file can't be read/parsed.
    fn read_disk_config(&self) -> Result<PartialScenarioConfig, String> {
        if self.config_path.is_empty() {
            return Err("No config path set".to_string());
        }
        let contents = std::fs::read_to_string(&self.config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        toml::from_str::<PartialScenarioConfig>(&contents)
            .map_err(|e| format!("Failed to parse config file: {}", e))
    }

    /// Persists a single task's current in-memory value to the on-disk config file.
    #[instrument(skip_all)]
    pub fn save_task_config(&self, task_name: &str) -> Result<(), String> {
        let leaf = self.leaf_config.as_ref().ok_or("No leaf config loaded")?;
        let mut disk = self.read_disk_config()?;
        let leaf_tasks = leaf.tasks.as_ref();

        let leaf_remote = leaf_tasks.and_then(|t| t.remote_sudo.as_ref()).and_then(|m| m.get(task_name)).cloned();
        let leaf_sftp = leaf_tasks.and_then(|t| t.sftp_copy.as_ref()).and_then(|m| m.get(task_name)).cloned();

        let disk_tasks = disk.tasks.get_or_insert_with(Default::default);

        match (leaf_remote, leaf_sftp) {
            (Some(task), _) => {
                disk_tasks.remote_sudo.get_or_insert_with(Default::default).insert(task_name.to_string(), task);
                if let Some(sftp) = disk_tasks.sftp_copy.as_mut() { sftp.remove(task_name); }
            }
            (None, Some(task)) => {
                disk_tasks.sftp_copy.get_or_insert_with(Default::default).insert(task_name.to_string(), task);
                if let Some(remote) = disk_tasks.remote_sudo.as_mut() { remote.remove(task_name); }
            }
            (None, None) => {
                if let Some(remote) = disk_tasks.remote_sudo.as_mut() { remote.remove(task_name); }
                if let Some(sftp) = disk_tasks.sftp_copy.as_mut() { sftp.remove(task_name); }
            }
        }

        let toml_string = toml::to_string_pretty(&disk)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(&self.config_path, toml_string)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        info!("Task '{}' saved", task_name);
        Ok(())
    }

    /// Reverts a single task's in-memory value to match the on-disk config file.
    #[instrument(skip_all)]
    pub fn discard_task_config(&mut self, task_name: &str) -> Result<(), String> {
        let disk = self.read_disk_config()?;
        let scenario = self.scenario.as_mut().ok_or("No scenario loaded")?;
        let leaf = self.leaf_config.as_mut().ok_or("No leaf config loaded")?;

        let disk_tasks = disk.tasks.as_ref();
        let disk_remote = disk_tasks.and_then(|t| t.remote_sudo.as_ref()).and_then(|m| m.get(task_name)).cloned();
        let disk_sftp = disk_tasks.and_then(|t| t.sftp_copy.as_ref()).and_then(|m| m.get(task_name)).cloned();

        let leaf_tasks = leaf.tasks.get_or_insert_with(Default::default);

        match (disk_remote, disk_sftp) {
            (Some(cfg), _) => {
                let task = Task::RemoteSudo {
                    description: cfg.description.clone().unwrap_or_default(),
                    error_message: cfg.error_message.clone().unwrap_or_default(),
                    remote_sudo: scenario_rs::scenario::remote_sudo::RemoteSudo::new(cfg.command.clone()),
                };
                scenario.tasks_mut().insert(task_name.to_string(), task);
                leaf_tasks.remote_sudo.get_or_insert_with(Default::default).insert(task_name.to_string(), cfg);
                if let Some(sftp) = leaf_tasks.sftp_copy.as_mut() { sftp.remove(task_name); }
            }
            (None, Some(cfg)) => {
                let task = Task::SftpCopy {
                    description: cfg.description.clone().unwrap_or_default(),
                    error_message: cfg.error_message.clone().unwrap_or_default(),
                    sftp_copy: scenario_rs::scenario::sftp_copy::SftpCopy {
                        source_path: cfg.source.clone(),
                        destination_path: cfg.destination.clone(),
                    },
                };
                scenario.tasks_mut().insert(task_name.to_string(), task);
                leaf_tasks.sftp_copy.get_or_insert_with(Default::default).insert(task_name.to_string(), cfg);
                if let Some(remote) = leaf_tasks.remote_sudo.as_mut() { remote.remove(task_name); }
            }
            (None, None) => {
                scenario.tasks_mut().remove(task_name);
                if let Some(remote) = leaf_tasks.remote_sudo.as_mut() { remote.remove(task_name); }
                if let Some(sftp) = leaf_tasks.sftp_copy.as_mut() { sftp.remove(task_name); }
            }
        }
        info!("Task '{}' discarded", task_name);
        Ok(())
    }

    /// Persists a single defined variable's current in-memory value to the on-disk config file.
    #[instrument(skip_all)]
    pub fn save_variable_config(&self, name: &str) -> Result<(), String> {
        let leaf = self.leaf_config.as_ref().ok_or("No leaf config loaded")?;
        let mut disk = self.read_disk_config()?;

        let leaf_value = leaf.variables.as_ref()
            .and_then(|v| v.defined.as_ref())
            .and_then(|d| d.get(name))
            .cloned();

        let disk_defined = disk.variables.get_or_insert_with(Default::default)
            .defined.get_or_insert_with(Default::default);

        match leaf_value {
            Some(value) => { disk_defined.insert(name.to_string(), value); }
            None => { disk_defined.remove(name); }
        }

        let toml_string = toml::to_string_pretty(&disk)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(&self.config_path, toml_string)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        info!("Variable '{}' saved", name);
        Ok(())
    }

    /// Reverts a single defined variable's in-memory value to match the on-disk config file.
    #[instrument(skip_all)]
    pub fn discard_variable_config(&mut self, name: &str) -> Result<(), String> {
        let disk = self.read_disk_config()?;
        let scenario = self.scenario.as_mut().ok_or("No scenario loaded")?;
        let leaf = self.leaf_config.as_mut().ok_or("No leaf config loaded")?;

        let disk_value = disk.variables.as_ref()
            .and_then(|v| v.defined.as_ref())
            .and_then(|d| d.get(name))
            .cloned();

        let leaf_defined = leaf.variables.get_or_insert_with(Default::default)
            .defined.get_or_insert_with(Default::default);

        match disk_value {
            Some(value) => {
                scenario.variables_mut().defined_mut().insert(name.to_string(), value.clone());
                leaf_defined.insert(name.to_string(), value);
            }
            None => {
                scenario.variables_mut().defined_mut().remove(name);
                leaf_defined.remove(name);
            }
        }
        info!("Variable '{}' discarded", name);
        Ok(())
    }
}

#[cfg(not(tarpaulin_include))]
impl ScenarioAppState {
    pub fn new(app_handle: &AppHandle) -> Self {
        Self {
            core: ScenarioAppStateCore {
                config_path: String::new(),
                scenario: None,
                leaf_config: None,
                is_executing: Arc::new(AtomicBool::new(false)),
                dry_run: false,
                execution_state_manager: None,
                state_storage: Box::new(FileStateStorage::new(
                    ScenarioAppStateCore::DEFAULT_STATE_FILE_PATH.to_string(),
                )),
            },
            app_handle: app_handle.clone(),
        }
    }

    /// Initializes the app state: sets up event listener and loads saved state.
    pub fn init(&mut self, frontend_rx: Receiver<AppEvent>) {
        trace::listen(frontend_rx, &self.app_handle, FrontendEventHandler);
        self.load_state();
    }

    /// Executes the currently loaded scenario in an async task.
    #[instrument(skip_all)]
    pub fn execute_scenario(&mut self) {
        if let Some(scenario) = self.scenario.as_ref().cloned() {
            let is_executing = self.is_executing.clone();
            let debug_mode = self.dry_run;

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
                let _guard = ExecutingGuard(is_executing);
                scenario.execute(Some(&state_manager), debug_mode);
            });
        } else {
            info!("No scenario loaded");
        }
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

/// Collects a batch of diffs: blocks for the first one, then drains
/// any additional diffs that arrive within the given timeout window.
/// Returns `None` when the channel is closed.
fn collect_batch(
    rx: &std::sync::mpsc::Receiver<scenario_rs::state::types::StateDiff>,
    batch_window: Duration,
) -> Option<Vec<scenario_rs::state::types::StateDiff>> {
    // Block until first diff arrives (or channel closes)
    let first = rx.recv().ok()?;

    let mut batch = vec![first];
    let deadline = Instant::now() + batch_window;

    // Collect more diffs within the window
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

    Some(batch)
}

/// Reads diffs from the channel, batches them over 100ms windows,
/// and emits them as a single Tauri event per batch.
#[cfg(not(tarpaulin_include))]
fn diff_batch_stream(
    rx: std::sync::mpsc::Receiver<scenario_rs::state::types::StateDiff>,
    app_handle: &AppHandle,
) {
    while let Some(batch) = collect_batch(&rx, Duration::from_millis(100)) {
        let _ = app_handle.emit("execution-diff", &batch);
    }
}

pub fn is_valid_config_path(path: &str) -> bool {
    let path = Path::new(path);
    path.exists() && path.is_file() && path.extension() == Some(OsStr::new("toml"))
}

#[cfg(test)]
mod tests {
    use crate::app::{
        build_initial_state, ConfigPathData, OnFailStepDTO, ScenarioAppStateConfig,
        ScenarioAppStateCore, StateStorage, StepDTO, TaskDTO,
    };
    use scenario_rs::scenario::{
        on_fail_step::OnFailStep, on_fail_steps::OnFailSteps, remote_sudo::RemoteSudo,
        sftp_copy::SftpCopy, step::Step, task::Task, Scenario,
    };
    use scenario_rs::state::types::{ExecutionStatus, StepStatus};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::Duration;

    /// In-memory implementation of StateStorage for testing.
    struct InMemoryStateStorage {
        content: RefCell<Option<String>>,
    }

    impl InMemoryStateStorage {
        fn empty() -> Self {
            Self {
                content: RefCell::new(None),
            }
        }

        fn with_content(json: &str) -> Self {
            Self {
                content: RefCell::new(Some(json.to_string())),
            }
        }

    }

    impl StateStorage for InMemoryStateStorage {
        fn read(&self) -> std::io::Result<String> {
            self.content
                .borrow()
                .clone()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no state"))
        }
        fn write(&self, content: &str) -> std::io::Result<()> {
            *self.content.borrow_mut() = Some(content.to_string());
            Ok(())
        }
    }

    // SAFETY: RefCell is not Sync, but we only use InMemoryStateStorage
    // from single-threaded test code. The Send bound is required by the
    // StateStorage trait (ScenarioAppStateCore lives in a Mutex<T>).
    unsafe impl Send for InMemoryStateStorage {}

    fn test_core_empty() -> ScenarioAppStateCore {
        ScenarioAppStateCore {
            config_path: String::new(),
            scenario: None,
            leaf_config: None,
            is_executing: Arc::new(AtomicBool::new(false)),
            dry_run: false,
            execution_state_manager: None,
            state_storage: Box::new(InMemoryStateStorage::empty()),
        }
    }

    fn test_core_with_storage(storage: InMemoryStateStorage) -> ScenarioAppStateCore {
        ScenarioAppStateCore {
            config_path: String::new(),
            scenario: None,
            leaf_config: None,
            is_executing: Arc::new(AtomicBool::new(false)),
            dry_run: false,
            execution_state_manager: None,
            state_storage: Box::new(storage),
        }
    }

    fn test_core_with_scenario() -> ScenarioAppStateCore {
        let scenario =
            Scenario::try_from("../../example_configs/example-scenario.toml").unwrap();
        ScenarioAppStateCore {
            config_path: "../../example_configs/example-scenario.toml".to_string(),
            scenario: Some(scenario),
            leaf_config: None,
            is_executing: Arc::new(AtomicBool::new(false)),
            dry_run: false,
            execution_state_manager: None,
            state_storage: Box::new(InMemoryStateStorage::empty()),
        }
    }

    fn test_core_with_scenario_and_storage(
        storage: InMemoryStateStorage,
    ) -> ScenarioAppStateCore {
        let scenario =
            Scenario::try_from("../../example_configs/example-scenario.toml").unwrap();
        ScenarioAppStateCore {
            config_path: "../../example_configs/example-scenario.toml".to_string(),
            scenario: Some(scenario),
            leaf_config: None,
            is_executing: Arc::new(AtomicBool::new(false)),
            dry_run: false,
            execution_state_manager: None,
            state_storage: Box::new(storage),
        }
    }

    /// Storage that always fails on write — for testing error branches.
    struct FailingWriteStorage;

    impl StateStorage for FailingWriteStorage {
        fn read(&self) -> std::io::Result<String> {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no state",
            ))
        }
        fn write(&self, _content: &str) -> std::io::Result<()> {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "write denied",
            ))
        }
    }

    // SAFETY: FailingWriteStorage has no interior mutability.
    unsafe impl Send for FailingWriteStorage {}

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
    fn test_collect_batch_returns_none_when_channel_closed() {
        use scenario_rs::state::types::StateDiff;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel::<StateDiff>();
        drop(tx);

        let result = super::collect_batch(&rx, Duration::from_millis(50));
        assert!(result.is_none());
    }

    #[test]
    fn test_collect_batch_returns_single_diff() {
        use scenario_rs::state::types::StateDiff;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel::<StateDiff>();
        tx.send(StateDiff::ExecutionStatusChanged {
            status: ExecutionStatus::Running,
        })
        .unwrap();
        drop(tx);

        let batch = super::collect_batch(&rx, Duration::from_millis(50));
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().len(), 1);
    }

    #[test]
    fn test_collect_batch_batches_multiple_diffs() {
        use scenario_rs::state::types::StateDiff;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel::<StateDiff>();
        tx.send(StateDiff::ExecutionStatusChanged {
            status: ExecutionStatus::Running,
        })
        .unwrap();
        tx.send(StateDiff::ExecutionStatusChanged {
            status: ExecutionStatus::Completed,
        })
        .unwrap();
        drop(tx);

        let batch = super::collect_batch(&rx, Duration::from_millis(100));
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().len(), 2);
    }

    #[test]
    fn test_collect_batch_respects_timeout_window() {
        use scenario_rs::state::types::StateDiff;
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel::<StateDiff>();

        // Given
        tx.send(StateDiff::ExecutionStatusChanged {
            status: ExecutionStatus::Running,
        })
        .unwrap();

        let tx_clone = tx.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            let _ = tx_clone.send(StateDiff::ExecutionStatusChanged {
                status: ExecutionStatus::Completed,
            });
        });

        // When
        let batch = super::collect_batch(&rx, Duration::from_millis(50));
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().len(), 1);
    }

    #[test]
    fn test_from_core_without_scenario_produces_empty_config() {
        let core = test_core_empty();
        let config = ScenarioAppStateConfig::from(&core);
        assert!(config.last_config_path.is_empty());
        assert!(config.config_paths.is_empty());
    }

    #[test]
    fn test_from_core_with_scenario_includes_writable_variables() {
        // Given
        let mut core = test_core_with_scenario();
        let var_names: Vec<String> = core
            .scenario
            .as_ref()
            .unwrap()
            .variables()
            .required()
            .iter()
            .filter(|(_, rv)| rv.not_read_only())
            .map(|(name, _)| name.to_string())
            .collect();
        if let Some(first_var) = var_names.first() {
            core.update_required_variables(HashMap::from([(
                first_var.clone(),
                "test_val".to_string(),
            )]));
        }
        // When
        let config = ScenarioAppStateConfig::from(&core);

        // Then
        assert_eq!(
            config.last_config_path,
            "../../example_configs/example-scenario.toml"
        );
        assert!(config
            .config_paths
            .contains_key("../../example_configs/example-scenario.toml"));
    }

    #[test]
    fn test_load_state_empty_storage_is_noop() {
        let mut core = test_core_empty();
        core.load_state();
        assert!(core.config_path.is_empty());
        assert!(core.scenario.is_none());
    }

    #[test]
    fn test_load_state_invalid_json_is_noop() {
        let storage = InMemoryStateStorage::with_content("not valid json");
        let mut core = test_core_with_storage(storage);
        core.load_state();
        assert!(core.config_path.is_empty());
        assert!(core.scenario.is_none());
    }

    #[test]
    fn test_load_state_restores_config_path_and_scenario() {
        let state = ScenarioAppStateConfig {
            last_config_path: "../../example_configs/example-scenario.toml".to_string(),
            config_paths: HashMap::new(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let storage = InMemoryStateStorage::with_content(&json);
        let mut core = test_core_with_storage(storage);

        core.load_state();

        assert_eq!(
            core.config_path,
            "../../example_configs/example-scenario.toml"
        );
        assert!(core.scenario.is_some());
    }

    #[test]
    fn test_load_state_restores_required_variables_from_state() {
        let mut config_paths = HashMap::new();
        config_paths.insert(
            "../../example_configs/example-scenario.toml".to_string(),
            ConfigPathData {
                required_variables: HashMap::from([
                    ("MY_VAR".to_string(), "my_value".to_string()),
                ]),
            },
        );
        let state = ScenarioAppStateConfig {
            last_config_path: "../../example_configs/example-scenario.toml".to_string(),
            config_paths,
        };
        let json = serde_json::to_string(&state).unwrap();
        let storage = InMemoryStateStorage::with_content(&json);
        let mut core = test_core_with_storage(storage);

        core.load_state();

        assert!(core.scenario.is_some());
    }

    #[test]
    fn test_load_state_with_bad_config_path_sets_scenario_none() {
        let state = ScenarioAppStateConfig {
            last_config_path: "/nonexistent/path.toml".to_string(),
            config_paths: HashMap::new(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let storage = InMemoryStateStorage::with_content(&json);
        let mut core = test_core_with_storage(storage);

        core.load_state();

        assert_eq!(core.config_path, "/nonexistent/path.toml");
        assert!(core.scenario.is_none());
    }

    #[test]
    fn test_save_state_to_empty_storage() {
        // Given
        let storage = InMemoryStateStorage::empty();
        let mut core = test_core_with_scenario_and_storage(storage);

        // When
        core.save_state();

        // Then
        let content = core.state_storage.read().unwrap();
        let saved: ScenarioAppStateConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(
            saved.last_config_path,
            "../../example_configs/example-scenario.toml"
        );
    }

    #[test]
    fn test_save_state_merges_with_existing() {
        // Given
        let mut existing_paths = HashMap::new();
        existing_paths.insert(
            "/old/config.toml".to_string(),
            ConfigPathData {
                required_variables: HashMap::from([("OLD_VAR".to_string(), "old".to_string())]),
            },
        );
        let existing = ScenarioAppStateConfig {
            last_config_path: "/old/config.toml".to_string(),
            config_paths: existing_paths,
        };
        let existing_json = serde_json::to_string_pretty(&existing).unwrap();
        let storage = InMemoryStateStorage::with_content(&existing_json);
        let mut core = test_core_with_scenario_and_storage(storage);

        core.save_state();

        // Then
        let content = core.state_storage.read().unwrap();
        let saved: ScenarioAppStateConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(
            saved.last_config_path,
            "../../example_configs/example-scenario.toml"
        );
        assert!(saved.config_paths.contains_key("/old/config.toml"));
    }

    #[test]
    fn test_save_state_overwrites_corrupt_existing() {
        let storage = InMemoryStateStorage::with_content("not valid json");
        let mut core = test_core_with_scenario_and_storage(storage);

        core.save_state();

        let content = core.state_storage.read().unwrap();
        let saved: ScenarioAppStateConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(
            saved.last_config_path,
            "../../example_configs/example-scenario.toml"
        );
    }

    #[test]
    fn test_load_config_valid_path() {
        let mut core = test_core_empty();
        core.load_config("../../example_configs/example-scenario.toml");
        assert_eq!(
            core.config_path,
            "../../example_configs/example-scenario.toml"
        );
        assert!(core.scenario.is_some());
    }

    #[test]
    fn test_load_config_invalid_path() {
        let mut core = test_core_empty();
        core.load_config("/nonexistent/path.toml");
        assert_eq!(core.config_path, "/nonexistent/path.toml");
        assert!(core.scenario.is_none());
    }

    #[test]
    fn test_load_config_merges_state_from_storage() {
        let mut config_paths = HashMap::new();
        config_paths.insert(
            "../../example_configs/example-scenario.toml".to_string(),
            ConfigPathData {
                required_variables: HashMap::from([
                    ("SAVED_VAR".to_string(), "saved_value".to_string()),
                ]),
            },
        );
        let state = ScenarioAppStateConfig {
            last_config_path: "../../example_configs/example-scenario.toml".to_string(),
            config_paths,
        };
        let json = serde_json::to_string(&state).unwrap();
        let storage = InMemoryStateStorage::with_content(&json);
        let mut core = test_core_with_storage(storage);

        core.load_config("../../example_configs/example-scenario.toml");

        assert!(core.scenario.is_some());
    }

    #[test]
    fn test_clear_state_writes_empty_state() {
        let storage = InMemoryStateStorage::with_content("old content");
        let mut core = test_core_with_storage(storage);

        core.clear_state();

        let content = core.state_storage.read().unwrap();
        let cleared: ScenarioAppStateConfig = serde_json::from_str(&content).unwrap();
        assert!(cleared.last_config_path.is_empty());
        assert!(cleared.config_paths.is_empty());
    }

    #[test]
    fn test_get_execution_state_none_when_no_manager() {
        let core = test_core_empty();
        assert!(core.get_execution_state().is_none());
    }

    #[test]
    fn test_get_execution_state_returns_snapshot() {
        use scenario_rs::state::{types::ExecutionState, ExecutionStateManager};

        let initial = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![],
        };
        let (tx, _rx) = std::sync::mpsc::channel();
        let manager = Arc::new(ExecutionStateManager::new(initial, tx));
        let mut core = test_core_empty();
        core.execution_state_manager = Some(manager);

        let state = core.get_execution_state();
        assert!(state.is_some());
        assert_eq!(state.unwrap().status, ExecutionStatus::Running);
    }

    #[test]
    fn test_get_required_variables_empty_when_no_scenario() {
        let core = test_core_empty();
        let vars = core.get_required_variables();
        assert!(vars.is_empty());
    }

    #[test]
    fn test_get_required_variables_returns_dtos_when_scenario_loaded() {
        let core = test_core_with_scenario();
        let vars = core.get_required_variables();

        // Then
        assert!(!vars.is_empty());
        for (_, dto) in &vars {
            assert!(!dto.label.is_empty());
        }
    }

    #[test]
    fn test_update_required_variables_noop_when_no_scenario() {
        let mut core = test_core_empty();
        core.update_required_variables(HashMap::from([
            ("KEY".to_string(), "VALUE".to_string()),
        ]));
        assert!(core.scenario.is_none());
    }

    #[test]
    fn test_update_required_variables_updates_scenario() {
        let mut core = test_core_with_scenario();
        let var_names: Vec<String> = core
            .scenario
            .as_ref()
            .unwrap()
            .variables()
            .required()
            .iter()
            .map(|(name, _)| name.to_string())
            .collect();

        if let Some(first_var) = var_names.first() {
            core.update_required_variables(HashMap::from([
                (first_var.clone(), "updated_value".to_string()),
            ]));

            let updated = core
                .scenario
                .as_ref()
                .unwrap()
                .variables()
                .required()
                .iter()
                .find(|(name, _)| *name == first_var)
                .map(|(_, rv)| rv.value().to_string());

            assert_eq!(updated, Some("updated_value".to_string()));
        }
    }

    #[test]
    fn test_get_tasks_empty_when_no_scenario() {
        let core = test_core_empty();
        let tasks = core.get_tasks();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_get_tasks_returns_dtos_when_scenario_loaded() {
        let core = test_core_with_scenario();
        let tasks = core.get_tasks();
        assert!(!tasks.is_empty());
    }

    #[test]
    fn test_get_resolved_variables_empty_when_no_scenario() {
        let mut core = test_core_empty();
        let vars = core.get_resolved_variables();
        assert!(vars.is_empty());
    }

    #[test]
    fn test_get_resolved_variables_returns_resolved_when_loaded() {
        let mut core = test_core_with_scenario();
        let vars = core.get_resolved_variables();

        // Then
        assert!(!vars.is_empty());
    }

    #[test]
    fn test_get_steps_empty_when_no_scenario() {
        let core = test_core_empty();
        let steps = core.get_steps();
        assert!(steps.is_empty());
    }

    #[test]
    fn test_get_steps_returns_dtos_when_scenario_loaded() {
        let core = test_core_with_scenario();
        let steps = core.get_steps();
        assert!(!steps.is_empty());
        for step in &steps {
            assert!(!step.task.description.is_empty());
        }
    }

    #[test]
    fn test_save_state_with_write_failure_does_not_panic() {
        let mut core = ScenarioAppStateCore {
            config_path: String::new(),
            scenario: None,
            leaf_config: None,
            is_executing: Arc::new(AtomicBool::new(false)),
            dry_run: false,
            execution_state_manager: None,
            state_storage: Box::new(FailingWriteStorage),
        };

        // When & Then
        core.save_state();
    }

    #[test]
    fn test_clear_state_with_write_failure_does_not_panic() {
        let mut core = ScenarioAppStateCore {
            config_path: String::new(),
            scenario: None,
            leaf_config: None,
            is_executing: Arc::new(AtomicBool::new(false)),
            dry_run: false,
            execution_state_manager: None,
            state_storage: Box::new(FailingWriteStorage),
        };

        // When & Then
        core.clear_state();
    }

    #[test]
    fn test_get_resolved_variables_error_returns_empty() {
        // Given
        let mut core = test_core_with_scenario();
        let var_names: Vec<String> = core
            .scenario
            .as_ref()
            .unwrap()
            .variables()
            .required()
            .iter()
            .map(|(name, _)| name.to_string())
            .collect();
        if let Some(first_var) = var_names.first() {
            core.update_required_variables(HashMap::from([(
                first_var.clone(),
                "{nonexistent_placeholder}".to_string(),
            )]));
        }

        // When
        let vars = core.get_resolved_variables();

        // Then
        assert!(vars.is_empty());
    }

    #[test]
    fn test_collect_batch_with_zero_timeout_returns_first_diff_only() {
        use scenario_rs::state::types::StateDiff;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel::<StateDiff>();

        // Given
        tx.send(StateDiff::ExecutionStatusChanged {
            status: ExecutionStatus::Running,
        })
        .unwrap();
        tx.send(StateDiff::ExecutionStatusChanged {
            status: ExecutionStatus::Completed,
        })
        .unwrap();

        // When
        let batch = super::collect_batch(&rx, Duration::ZERO);

        // Then
        assert!(batch.is_some());
        assert!(!batch.unwrap().is_empty());
    }

    #[test]
    fn test_is_valid_config_path_with_existing_toml() {
        // Given
        let path = "../../example_configs/example-scenario.toml";

        // When
        let result = super::is_valid_config_path(path);

        // Then
        assert!(result);
    }

    #[test]
    fn test_is_valid_config_path_with_nonexistent_file() {
        // Given
        let path = "nonexistent/file.toml";

        // When
        let result = super::is_valid_config_path(path);

        // Then
        assert!(!result);
    }

    #[test]
    fn test_is_valid_config_path_with_toml_extension() {
        // Given
        let path = "../../Cargo.toml";

        // When
        let result = super::is_valid_config_path(path);

        // Then
        assert!(result);
    }

    #[test]
    fn test_is_valid_config_path_with_directory() {
        // Given
        let path = "../../example_configs";

        // When
        let result = super::is_valid_config_path(path);

        // Then
        assert!(!result);
    }

    #[test]
    fn test_is_valid_config_path_with_non_toml_file() {
        // Given
        let path = "../../README.md";

        // When
        let result = super::is_valid_config_path(path);

        // Then
        assert!(!result);
    }

    #[test]
    fn test_executing_guard_resets_flag_on_drop() {
        use std::sync::atomic::Ordering;

        // Given
        let flag = Arc::new(AtomicBool::new(true));
        let guard = super::ExecutingGuard(Arc::clone(&flag));
        assert!(flag.load(Ordering::SeqCst));

        // When
        drop(guard);

        // Then
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn test_load_config_with_scenario_build_failure() {
        // Given — valid TOML that references a non-existent task in steps
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("bad-scenario.toml");
        std::fs::write(
            &config_path,
            r#"
            [credentials]
            username = "user"

            [server]
            host = "host"

            [[steps]]
            name = "bad_step"
            task = "nonexistent_task"

            [tasks]
            "#,
        )
        .unwrap();
        let mut core = test_core_empty();

        // When
        core.load_config(config_path.to_str().unwrap());

        // Then — TOML loads but Scenario::try_from fails
        assert!(core.scenario.is_none());
        assert!(core.leaf_config.is_some());
    }

    fn test_core_loaded_from_file() -> (ScenarioAppStateCore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("test-scenario.toml");
        std::fs::write(
            &config_path,
            r#"
            [credentials]
            username = "user"

            [server]
            host = "host"

            [[steps]]
            name = "step1"
            task = "my_task"

            [tasks.remote_sudo.my_task]
            command = "echo hello"
            "#,
        )
        .unwrap();
        let mut core = test_core_empty();
        core.load_config(config_path.to_str().unwrap());
        assert!(core.scenario.is_some());
        assert!(core.leaf_config.is_some());
        (core, dir)
    }

    #[test]
    fn test_update_task_remote_sudo() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();

        // When
        let result = core.update_task(
            "my_task".to_string(),
            TaskDTO {
                description: "Updated desc".to_string(),
                error_message: "Updated err".to_string(),
                task_type: "RemoteSudo".to_string(),
                command: Some("echo updated".to_string()),
                source_path: None,
                destination_path: None,
            },
        );

        // Then
        assert!(result.is_ok());
        let task = core.scenario.as_ref().unwrap().tasks().get("my_task").unwrap();
        assert_eq!(task.description(), "Updated desc");
    }

    #[test]
    fn test_update_task_sftp_copy() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();

        // When
        let result = core.update_task(
            "upload".to_string(),
            TaskDTO {
                description: "Upload file".to_string(),
                error_message: "Failed".to_string(),
                task_type: "SftpCopy".to_string(),
                command: None,
                source_path: Some("/local/path".to_string()),
                destination_path: Some("/remote/path".to_string()),
            },
        );

        // Then
        assert!(result.is_ok());
        assert!(core.scenario.as_ref().unwrap().tasks().contains_key("upload"));
    }

    #[test]
    fn test_update_task_unknown_type() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();

        // When
        let result = core.update_task(
            "x".to_string(),
            TaskDTO {
                description: String::new(),
                error_message: String::new(),
                task_type: "UnknownType".to_string(),
                command: None,
                source_path: None,
                destination_path: None,
            },
        );

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown task type"));
    }

    #[test]
    fn test_update_task_no_scenario_returns_error() {
        // Given
        let mut core = test_core_empty();

        // When
        let result = core.update_task(
            "x".to_string(),
            TaskDTO {
                description: String::new(),
                error_message: String::new(),
                task_type: "RemoteSudo".to_string(),
                command: Some("echo".to_string()),
                source_path: None,
                destination_path: None,
            },
        );

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_update_defined_variable_success() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();

        // When
        let result = core.update_defined_variable("new_var".to_string(), "new_val".to_string());

        // Then
        assert!(result.is_ok());
        assert_eq!(
            core.scenario.as_ref().unwrap().variables().defined().get("new_var"),
            Some(&"new_val".to_string())
        );
    }

    #[test]
    fn test_update_defined_variable_no_scenario() {
        // Given
        let mut core = test_core_empty();

        // When
        let result = core.update_defined_variable("x".to_string(), "y".to_string());

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn test_get_config_diff_empty_when_unchanged() {
        // Given
        let (core, _dir) = test_core_loaded_from_file();

        // When
        let diff = core.get_config_diff();

        // Then
        assert!(diff.modified_tasks.is_empty());
        assert!(diff.modified_variables.is_empty());
    }

    #[test]
    fn test_get_config_diff_detects_added_variable() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();

        // When
        core.update_defined_variable("added_var".to_string(), "value".to_string()).unwrap();
        let diff = core.get_config_diff();

        // Then
        assert!(diff.modified_variables.contains(&"added_var".to_string()));
    }

    #[test]
    fn test_get_config_diff_detects_modified_task() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();
        let task_name = core
            .leaf_config
            .as_ref()
            .and_then(|c| c.tasks.as_ref())
            .and_then(|t| t.remote_sudo.as_ref())
            .and_then(|m| m.keys().next().cloned())
            .expect("fixture must have at least one remote_sudo task");

        // When
        core.update_task(
            task_name.clone(),
            TaskDTO {
                description: "new description".to_string(),
                error_message: "err".to_string(),
                task_type: "RemoteSudo".to_string(),
                command: Some("echo hi".to_string()),
                source_path: None,
                destination_path: None,
            },
        )
        .unwrap();
        let diff = core.get_config_diff();

        // Then
        assert!(diff.modified_tasks.contains(&task_name));
    }

    #[test]
    fn test_get_config_diff_empty_without_leaf() {
        // Given
        let core = test_core_empty();

        // When
        let diff = core.get_config_diff();

        // Then
        assert!(diff.modified_tasks.is_empty());
        assert!(diff.modified_variables.is_empty());
    }

    #[test]
    fn test_get_config_diff_empty_without_path() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();
        core.config_path.clear();

        // When
        let diff = core.get_config_diff();

        // Then
        assert!(diff.modified_tasks.is_empty());
        assert!(diff.modified_variables.is_empty());
    }

    #[test]
    fn test_get_config_diff_empty_when_file_missing() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();
        core.config_path = "/nonexistent/path/to/config.toml".to_string();

        // When
        let diff = core.get_config_diff();

        // Then
        assert!(diff.modified_tasks.is_empty());
        assert!(diff.modified_variables.is_empty());
    }

    #[test]
    fn test_get_config_diff_empty_when_file_unparseable() {
        // Given
        let (core, _dir) = test_core_loaded_from_file();
        std::fs::write(&core.config_path, "this is not valid toml ][{{").unwrap();

        // When
        let diff = core.get_config_diff();

        // Then
        assert!(diff.modified_tasks.is_empty());
        assert!(diff.modified_variables.is_empty());
    }

    #[test]
    fn test_discard_config_changes_reloads_from_disk() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();
        core.update_defined_variable("tmp_var".to_string(), "val".to_string()).unwrap();
        assert!(!core.get_config_diff().modified_variables.is_empty());

        // When
        core.discard_config_changes();

        // Then
        let diff = core.get_config_diff();
        assert!(diff.modified_tasks.is_empty());
        assert!(diff.modified_variables.is_empty());
    }

    #[test]
    fn test_save_config_success() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();
        core.update_defined_variable("save_var".to_string(), "save_val".to_string()).unwrap();
        assert!(!core.get_config_diff().modified_variables.is_empty());

        // When
        let result = core.save_config();

        // Then
        assert!(result.is_ok());
        let diff = core.get_config_diff();
        assert!(diff.modified_tasks.is_empty());
        assert!(diff.modified_variables.is_empty());
    }

    #[test]
    fn test_save_config_no_leaf_returns_error() {
        // Given
        let core = test_core_empty();

        // When
        let result = core.save_config();

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No leaf config loaded"));
    }

    #[test]
    fn test_save_config_no_config_path_returns_error() {
        // Given
        let mut core = test_core_empty();
        core.leaf_config = Some(Default::default());

        // When
        let result = core.save_config();

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No config path set"));
    }

    // ── save_task_config / discard_task_config / save_variable_config / discard_variable_config ──

    #[test]
    fn test_save_task_config_persists_only_that_task() {
        // Given: fixture has `my_task`. Modify it in memory and also add a dirty variable.
        let (mut core, _dir) = test_core_loaded_from_file();
        core.update_task(
            "my_task".to_string(),
            TaskDTO {
                description: "updated".to_string(),
                error_message: String::new(),
                task_type: "RemoteSudo".to_string(),
                command: Some("echo new".to_string()),
                source_path: None,
                destination_path: None,
            },
        )
        .unwrap();
        core.update_defined_variable("vv".to_string(), "val".to_string()).unwrap();

        // When
        core.save_task_config("my_task").unwrap();

        // Then: diff shows task is clean, variable still dirty
        let diff = core.get_config_diff();
        assert!(!diff.modified_tasks.contains(&"my_task".to_string()));
        assert!(diff.modified_variables.contains(&"vv".to_string()));
    }

    #[test]
    fn test_save_task_config_removes_task_missing_in_leaf() {
        // Given: pretend leaf has no tasks for this name — disk still has it
        let (mut core, _dir) = test_core_loaded_from_file();
        if let Some(leaf) = core.leaf_config.as_mut() {
            if let Some(tasks) = leaf.tasks.as_mut() {
                if let Some(rs) = tasks.remote_sudo.as_mut() { rs.remove("my_task"); }
            }
        }

        // When
        core.save_task_config("my_task").unwrap();

        // Then: disk no longer has the task
        let disk = core.read_disk_config().unwrap();
        let remote = disk.tasks.as_ref().and_then(|t| t.remote_sudo.as_ref());
        assert!(remote.map_or(true, |m| !m.contains_key("my_task")));
    }

    #[test]
    fn test_save_task_config_errors_without_leaf() {
        let core = test_core_empty();
        let err = core.save_task_config("x").unwrap_err();
        assert!(err.contains("No leaf config loaded"));
    }

    #[test]
    fn test_save_task_config_errors_without_path() {
        let mut core = test_core_empty();
        core.leaf_config = Some(Default::default());
        let err = core.save_task_config("x").unwrap_err();
        assert!(err.contains("No config path set"));
    }

    #[test]
    fn test_discard_task_config_restores_disk_value() {
        // Given
        let (mut core, _dir) = test_core_loaded_from_file();
        core.update_task(
            "my_task".to_string(),
            TaskDTO {
                description: "tmp".to_string(),
                error_message: String::new(),
                task_type: "RemoteSudo".to_string(),
                command: Some("echo changed".to_string()),
                source_path: None,
                destination_path: None,
            },
        )
        .unwrap();
        assert!(core.get_config_diff().modified_tasks.contains(&"my_task".to_string()));

        // When
        core.discard_task_config("my_task").unwrap();

        // Then
        assert!(!core.get_config_diff().modified_tasks.contains(&"my_task".to_string()));
    }

    #[test]
    fn test_discard_task_config_removes_task_missing_on_disk() {
        // Given: add a new task in memory that doesn't exist on disk
        let (mut core, _dir) = test_core_loaded_from_file();
        core.update_task(
            "ghost".to_string(),
            TaskDTO {
                description: "g".to_string(),
                error_message: String::new(),
                task_type: "RemoteSudo".to_string(),
                command: Some("echo g".to_string()),
                source_path: None,
                destination_path: None,
            },
        )
        .unwrap();

        // When
        core.discard_task_config("ghost").unwrap();

        // Then
        assert!(!core.scenario.as_ref().unwrap().tasks().contains_key("ghost"));
    }

    #[test]
    fn test_discard_task_config_errors_without_path() {
        let mut core = test_core_empty();
        let err = core.discard_task_config("x").unwrap_err();
        assert!(err.contains("No config path set"));
    }

    #[test]
    fn test_save_variable_config_persists_only_that_variable() {
        let (mut core, _dir) = test_core_loaded_from_file();
        core.update_defined_variable("a".to_string(), "1".to_string()).unwrap();
        core.update_defined_variable("b".to_string(), "2".to_string()).unwrap();

        core.save_variable_config("a").unwrap();

        let diff = core.get_config_diff();
        assert!(!diff.modified_variables.contains(&"a".to_string()));
        assert!(diff.modified_variables.contains(&"b".to_string()));
    }

    #[test]
    fn test_save_variable_config_removes_missing_variable() {
        let (mut core, _dir) = test_core_loaded_from_file();
        core.update_defined_variable("a".to_string(), "1".to_string()).unwrap();
        core.save_variable_config("a").unwrap();
        // Remove from leaf, then save — should drop from disk
        if let Some(v) = core.leaf_config.as_mut().and_then(|c| c.variables.as_mut()).and_then(|v| v.defined.as_mut()) {
            v.remove("a");
        }
        core.save_variable_config("a").unwrap();

        let disk = core.read_disk_config().unwrap();
        let defined = disk.variables.as_ref().and_then(|v| v.defined.as_ref());
        assert!(defined.map_or(true, |d| !d.contains_key("a")));
    }

    #[test]
    fn test_save_variable_config_errors_without_leaf() {
        let core = test_core_empty();
        let err = core.save_variable_config("x").unwrap_err();
        assert!(err.contains("No leaf config loaded"));
    }

    #[test]
    fn test_discard_variable_config_restores_disk_value() {
        let (mut core, _dir) = test_core_loaded_from_file();
        core.update_defined_variable("new".to_string(), "v".to_string()).unwrap();
        assert!(core.get_config_diff().modified_variables.contains(&"new".to_string()));

        core.discard_variable_config("new").unwrap();

        assert!(!core.get_config_diff().modified_variables.contains(&"new".to_string()));
        assert!(core.scenario.as_ref().unwrap().variables().defined().get("new").is_none());
    }

    #[test]
    fn test_discard_variable_config_errors_without_path() {
        let mut core = test_core_empty();
        let err = core.discard_variable_config("x").unwrap_err();
        assert!(err.contains("No config path set"));
    }

    #[test]
    fn test_read_disk_config_errors_on_missing_file() {
        let mut core = test_core_empty();
        core.config_path = "/nonexistent/file.toml".to_string();
        let err = core.read_disk_config().unwrap_err();
        assert!(err.contains("Failed to read"));
    }

    #[test]
    fn test_read_disk_config_errors_on_bad_toml() {
        let (mut core, _dir) = test_core_loaded_from_file();
        std::fs::write(&core.config_path, "][not toml").unwrap();
        let err = core.read_disk_config().unwrap_err();
        assert!(err.contains("Failed to parse"));
    }
}
