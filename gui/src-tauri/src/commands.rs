use crate::{
    app::{RequiredVariableDTO, ScenarioAppState, StepDTO, TaskDTO},
    utils::SafeLock,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{atomic::Ordering, Mutex},
};
use tauri::State;
use tracing::warn;

#[tauri::command(async)]
pub fn save_state(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.safe_lock();
    state.save_state();
}

#[tauri::command(async)]
pub fn get_config_path(state: State<'_, Mutex<ScenarioAppState>>) -> String {
    let state = state.safe_lock();
    state.config_path.clone()
}

#[tauri::command(async)]
pub fn load_config(config_path: &str, state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.safe_lock();
    state.load_config(config_path);
}

#[tauri::command(async)]
pub fn get_required_variables(
    state: State<'_, Mutex<ScenarioAppState>>,
) -> BTreeMap<String, RequiredVariableDTO> {
    let state = state.safe_lock();
    state.get_required_variables()
}

#[tauri::command(async)]
pub fn update_required_variables(
    required_variables: HashMap<String, String>,
    state: State<'_, Mutex<ScenarioAppState>>,
) {
    let mut state = state.safe_lock();
    state.update_required_variables(required_variables);
}

#[tauri::command(async)]
pub fn execute_scenario(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.safe_lock();
    if state.is_executing.load(Ordering::SeqCst) {
        warn!("Execution already in progress. Ignoring request.");
        return;
    }
    state.execute_scenario();
}

#[tauri::command(async)]
pub fn get_resolved_variables(
    state: State<'_, Mutex<ScenarioAppState>>,
) -> BTreeMap<String, String> {
    let mut state = state.safe_lock();
    state.get_resolved_variables()
}

#[tauri::command(async)]
pub fn clear_state(state: State<'_, Mutex<ScenarioAppState>>) -> Result<(), String> {
    let mut state = state.safe_lock();
    state.clear_state();
    Ok(())
}

#[tauri::command(async)]
pub fn get_tasks(state: State<'_, Mutex<ScenarioAppState>>) -> BTreeMap<String, TaskDTO> {
    let state = state.safe_lock();
    state.get_tasks()
}

#[tauri::command(async)]
pub fn get_steps(state: State<'_, Mutex<ScenarioAppState>>) -> Vec<StepDTO> {
    let state = state.safe_lock();
    state.get_steps()
}
