use crate::{lifecycle::LifecycleHandler, shared::SEPARATOR};
use scenario_rs::scenario::{variables::required::RequiredVariable, Scenario};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
};
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScenarioAppStateConfig {
    config_path: String,
    output_log: String,
    required_variables: HashMap<String, String>,
}

impl From<&ScenarioAppState> for ScenarioAppStateConfig {
    fn from(state: &ScenarioAppState) -> Self {
        Self {
            config_path: state.config_path.clone(),
            output_log: state.output_log.clone(),
            required_variables: state.scenario.as_ref().map_or_else(
                || HashMap::new(),
                |scenario| {
                    scenario
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
                        .collect()
                },
            ),
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
                self.config_path = loaded_state.config_path.clone();
                self.output_log = loaded_state.output_log;
                self.load_config(self.config_path.clone().as_str());

                if let Some(scenario) = self.scenario.as_mut() {
                    scenario
                        .variables_mut()
                        .required_mut()
                        .deref_mut()
                        .iter_mut()
                        .for_each(|required_variable| {
                            loaded_state
                                .required_variables
                                .get(required_variable.name())
                                .map(|value| required_variable.set_value(value.clone()));
                        });
                }
            }
        }
    }

    pub fn save_state(&mut self) {
        let state = ScenarioAppStateConfig::from(self.deref());
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            if let Err(error) = std::fs::write(Self::STATE_FILE_PATH, json) {
                self.log_message(format!(
                    "{SEPARATOR}\nFailed to save state: {error}\n{SEPARATOR}\n"
                ));
            }
        }
    }

    pub fn load_config(&mut self, config_path: &str) {
        match Scenario::try_from(config_path) {
            Ok(scenario) => {
                self.log_message(format!("{SEPARATOR}\nScenario loaded\n{SEPARATOR}\n"));
                self.scenario = Some(scenario);
            }
            Err(e) => {
                self.log_message(format!(
                    "{SEPARATOR}\nFailed to load scenario: {e}\n{SEPARATOR}\n"
                ));
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
