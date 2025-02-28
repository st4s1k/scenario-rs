use crate::{lifecycle::LifecycleHandler, shared::SEPARATOR};
use scenario_rs::scenario::{variables::required::RequiredVariable, Scenario};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
};
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigPathData {
    variables: HashMap<String, String>,
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
                let variables_map: HashMap<String, String> = scenario
                    .variables()
                    .required()
                    .deref()
                    .iter()
                    .map(|required_variable| {
                        (
                            required_variable.name().to_string(),
                            required_variable.value().to_string(),
                        )
                    })
                    .collect();

                let config_data = ConfigPathData {
                    variables: variables_map,
                    output_log: state.output_log.clone(),
                };

                config_paths.insert(state.config_path.clone(), config_data);
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
    pub(crate) scenario: Option<Scenario>,
    pub(crate) is_executing: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequiredVariableDTO {
    label: String,
    value: String,
}

impl From<&RequiredVariable> for RequiredVariableDTO {
    fn from(required_variable: &RequiredVariable) -> Self {
        Self {
            label: required_variable.label().to_string(),
            value: required_variable.value().to_string(),
        }
    }
}

impl ScenarioAppState {
    const STATE_FILE_PATH: &'static str = "scenario-app-state.json";

    pub fn new(app: AppHandle) -> Self {
        Self {
            config_path: String::new(),
            output_log: String::new(),
            app_handle: app,
            scenario: None,
            is_executing: false,
        }
    }

    pub fn load_state(&mut self) {
        if let Ok(json) = std::fs::read_to_string(Self::STATE_FILE_PATH) {
            if let Ok(loaded_state) = serde_json::from_str::<ScenarioAppStateConfig>(&json) {
                self.config_path = loaded_state.last_config_path.clone();
                self.load_config(self.config_path.clone().as_str());
                self.load_config_data_from_state(&loaded_state);
            }
        }
    }

    fn load_config_data_from_state(&mut self, state_config: &ScenarioAppStateConfig) {
        if let Some(config_data) = state_config.config_paths.get(&self.config_path) {
            self.output_log = config_data.output_log.clone();

            if let Some(scenario) = self.scenario.as_mut() {
                scenario
                    .variables_mut()
                    .required_mut()
                    .deref_mut()
                    .iter_mut()
                    .for_each(|required_variable| {
                        if let Some(value) = config_data.variables.get(required_variable.name()) {
                            required_variable.set_value(value.clone());
                        }
                    });
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
                    existing_state
                }
                Err(error) => {
                    self.log_message(format!(
                        "{SEPARATOR}\nFailed to parse existing state: {error}\n{SEPARATOR}\n"
                    ));
                    current_state
                }
            },
            Err(_) => current_state,
        };

        match serde_json::to_string_pretty(&final_state) {
            Ok(json) => {
                if let Err(error) = std::fs::write(Self::STATE_FILE_PATH, json) {
                    self.log_message(format!(
                        "{SEPARATOR}\nFailed to save state: {error}\n{SEPARATOR}\n"
                    ));
                }
            }
            Err(error) => {
                self.log_message(format!(
                    "{SEPARATOR}\nFailed to serialize state: {error}\n{SEPARATOR}\n"
                ));
            }
        }
    }

    pub fn load_config(&mut self, config_path: &str) {
        self.config_path = config_path.to_string();
        self.scenario = match Scenario::try_from(config_path) {
            Ok(scenario) => {
                self.log_message(format!("{SEPARATOR}\nScenario loaded\n{SEPARATOR}\n"));
                Some(scenario)
            }
            Err(e) => {
                self.log_message(format!(
                    "{SEPARATOR}\nFailed to load scenario: {e}\n{SEPARATOR}\n"
                ));
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
        let lifecycle_handler = LifecycleHandler::try_initialize(self.app_handle.clone());

        self.is_executing = true;

        let scenario = match self.scenario.as_ref() {
            Some(scenario) => scenario,
            None => {
                self.log_message(format!("{SEPARATOR}\nNo scenario loaded\n{SEPARATOR}\n"));
                return;
            }
        };

        match scenario.execute_with_lifecycle(lifecycle_handler) {
            Ok(_) => self.log_message(format!(
                "{SEPARATOR}\nScenario completed successfully!\n{SEPARATOR}\n"
            )),
            Err(e) => self.log_message(format!("{SEPARATOR}\nScenario failed: {e}\n{SEPARATOR}\n")),
        }

        self.is_executing = false;
    }

    pub fn get_required_variables(&self) -> BTreeMap<String, RequiredVariableDTO> {
        if let Some(scenario) = self.scenario.as_ref() {
            scenario
                .variables()
                .required()
                .iter()
                .map(|required_variable| {
                    (
                        required_variable.name().to_string(),
                        RequiredVariableDTO::from(required_variable),
                    )
                })
                .collect()
        } else {
            BTreeMap::new()
        }
    }

    fn log_message(&mut self, message: String) {
        self.output_log.push_str(&message);
        let _ = self.app_handle.emit("log-update", ());
    }

    pub fn clear_log(&mut self) {
        self.output_log.clear();
        let _ = self.app_handle.emit("log-update", ());
    }
}
