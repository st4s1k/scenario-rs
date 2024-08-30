use crate::app::ScenarioAppState;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Mutex,
};
use tauri::State;

#[tauri::command(async)]
pub fn get_config_path(state: State<'_, Mutex<ScenarioAppState>>) -> String {
    let state = state.lock().unwrap();
    state.config_path.clone()
}

#[tauri::command(async)]
pub fn get_log(state: State<'_, Mutex<ScenarioAppState>>) -> String {
    let state = state.lock().unwrap();
    state.output_log.clone()
}

#[tauri::command]
pub fn clear_log(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.lock().unwrap();
    state.clear_log();
}

#[tauri::command(async)]
pub fn load_config(
    config_path: &str,
    state: State<'_, Mutex<ScenarioAppState>>,
) -> BTreeMap<String, String> {
    let mut state = state.lock().unwrap();
    state
        .load_config(config_path)
        .as_deref()
        .cloned()
        .unwrap_or(BTreeMap::new())
}

#[tauri::command(async)]
pub fn execute_scenario(
    required_variables: HashMap<String, String>,
    state: State<'_, Mutex<ScenarioAppState>>,
) {
    let mut state = state.lock().unwrap();
    if state.is_executing {
        return;
    }
    state.required_variables = required_variables.clone();
    state.execute_scenario();
}
