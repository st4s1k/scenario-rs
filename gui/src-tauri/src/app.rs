use crate::{
    app_event::{AppEvent, AppEventChannel},
    scenario_event::ScenarioEventChannel,
};
use scenario_rs::scenario::{
    step::Step,
    task::Task,
    utils::SendEvent,
    variables::required::{RequiredVariable, VariableType},
    Scenario,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigPathData {
    required_variables: HashMap<String, String>,
    output_log: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScenarioAppStateConfig {
    last_config_path: String,
    config_paths: HashMap<String, ConfigPathData>,
}

impl From<&ScenarioAppState> for ScenarioAppStateConfig {
    fn from(state: &ScenarioAppState) -> Self {
        let mut config_paths = HashMap::new();

        if let Some(scenario) = &state.scenario {
            if !state.config_path.is_empty() {
                let required_variables: HashMap<String, String> = scenario
                    .variables()
                    .required()
                    .iter()
                    .filter(|(_, required_variable)| {
                        !required_variable.value().is_empty() && !required_variable.read_only()
                    })
                    .map(|(name, required_variable)| {
                        (name.to_string(), required_variable.value().to_string())
                    })
                    .collect();

                if !required_variables.is_empty() || !state.output_log.is_empty() {
                    let config_data = ConfigPathData {
                        required_variables,
                        output_log: state.output_log.clone(),
                    };

                    config_paths.insert(state.config_path.clone(), config_data);
                }
            }
        }

        Self {
            last_config_path: state.config_path.clone(),
            config_paths,
        }
    }
}

pub struct ScenarioAppState {
    pub(crate) config_path: String,
    pub(crate) output_log: String,
    pub(crate) app_handle: AppHandle,
    pub(crate) event_channel: Option<AppEventChannel>,
    pub(crate) scenario: Option<Scenario>,
    pub(crate) is_executing: Arc<AtomicBool>,
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskDTO {
    description: String,
    error_message: String,
    task_type: String,
    command: Option<String>,
    source_path: Option<String>,
    destination_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StepDTO {
    task: TaskDTO,
    on_fail_steps: Vec<TaskDTO>,
}

impl From<&Step> for StepDTO {
    fn from(step: &Step) -> Self {
        let on_fail_tasks: Vec<TaskDTO> = step
            .on_fail_steps()
            .iter()
            .map(|task| TaskDTO::from(task))
            .collect();

        Self {
            task: TaskDTO::from(step.task()),
            on_fail_steps: on_fail_tasks,
        }
    }
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

impl ScenarioAppState {
    const STATE_FILE_PATH: &'static str = "scenario-app-state.json";

    pub fn new(app_handle: &AppHandle) -> Self {
        Self {
            config_path: String::new(),
            output_log: String::new(),
            app_handle: app_handle.clone(),
            event_channel: None,
            scenario: None,
            is_executing: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn init(&mut self) {
        self.init_event_channel();
        self.load_state();
    }

    pub fn init_event_channel(&mut self) {
        let event_channel = AppEventChannel::new(&self.app_handle);
        self.event_channel = Some(event_channel);
    }

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

    fn load_config_data_from_state(&mut self, state_config: &ScenarioAppStateConfig) {
        if let Some(config_data) = state_config.config_paths.get(&self.config_path) {
            if !config_data.output_log.is_empty() {
                self.output_log = config_data.output_log.clone();
            }

            if let Some(scenario) = self.scenario.as_mut() {
                scenario
                    .variables_mut()
                    .required_mut()
                    .upsert(config_data.required_variables.clone());
            }
        }
    }

    pub fn save_state(&mut self) {
        let current_state = ScenarioAppStateConfig::from(self.deref());

        let final_state = match std::fs::read_to_string(Self::STATE_FILE_PATH) {
            Ok(json) => match serde_json::from_str::<ScenarioAppStateConfig>(&json) {
                Ok(mut existing_state) => {
                    existing_state.last_config_path = current_state.last_config_path.clone();
                    existing_state
                        .config_paths
                        .extend(current_state.config_paths.clone());
                    self.send_event(AppEvent::StateLoaded);
                    existing_state
                }
                Err(error) => {
                    self.send_event(AppEvent::FailedToDeserializeState(Arc::new(error)));
                    current_state
                }
            },
            Err(error) => {
                self.send_event(AppEvent::FailedToLoadState(Arc::new(error)));
                current_state
            }
        };

        match serde_json::to_string_pretty(&final_state) {
            Ok(json) => match std::fs::write(Self::STATE_FILE_PATH, json) {
                Ok(_) => {
                    self.send_event(AppEvent::StateSuccessfullySaved);
                }
                Err(error) => {
                    self.send_event(AppEvent::FailedToSaveState(Arc::new(error)));
                }
            },
            Err(error) => {
                self.send_event(AppEvent::FailedToSerializeState(Arc::new(error)));
            }
        }
    }

    pub fn load_config(&mut self, config_path: &str) {
        self.config_path = config_path.to_string();
        self.scenario = match Scenario::try_from(config_path) {
            Ok(scenario) => {
                self.send_event(AppEvent::ConfigLoaded(config_path.to_string()));
                Some(scenario)
            }
            Err(error) => {
                self.send_event(AppEvent::FailedToLoadConfig(Arc::new(error)));
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

    pub fn execute_scenario(&mut self) {
        if let Some(scenario) = self.scenario.as_ref().cloned() {
            let event_channel = ScenarioEventChannel::new(&self.app_handle);
            let tx = event_channel.sender().clone();
            let is_executing = self.is_executing.clone();
            tauri::async_runtime::spawn(async move {
                is_executing.store(true, Ordering::SeqCst);
                scenario.execute(tx);
                is_executing.store(false, Ordering::SeqCst);
            });
        } else {
            self.send_event(AppEvent::NoScenarioLoaded);
        }
    }

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

    pub fn update_required_variables(&mut self, required_variables: HashMap<String, String>) {
        if let Some(scenario) = self.scenario.as_mut() {
            scenario
                .variables_mut()
                .required_mut()
                .upsert(required_variables);
            self.send_event(AppEvent::RequiredVariablesUpdated);
        } else {
            self.send_event(AppEvent::NoScenarioLoaded);
        }
    }

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

    pub fn get_resolved_variables(&mut self) -> BTreeMap<String, String> {
        if let Some(scenario) = &self.scenario {
            match scenario.variables().resolved() {
                Ok(resolved) => resolved
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect(),
                Err(error) => {
                    self.send_event(AppEvent::FailedToGetResolvedVariables(Arc::new(error)));
                    BTreeMap::new()
                }
            }
        } else {
            BTreeMap::new()
        }
    }

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

    pub fn clear_log(&mut self) {
        self.send_event(AppEvent::ClearLog);
    }

    pub fn clear_state(&mut self) {
        let empty_state = ScenarioAppStateConfig {
            last_config_path: String::new(),
            config_paths: HashMap::new(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&empty_state) {
            if let Err(error) = std::fs::write(Self::STATE_FILE_PATH, json) {
                self.send_event(AppEvent::FailedToClearState(Arc::new(error)));
            }
        }

        self.send_event(AppEvent::StateCleared);
    }

    fn send_event(&self, event: AppEvent) {
        if let Some(event_channel) = &self.event_channel {
            event_channel.sender().send_event(event);
        } else {
            eprintln!("App event channel is not initialized: {:?}", event);
        }
    }
}
