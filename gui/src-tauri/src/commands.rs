use crate::app::{RequiredVariableDTO, ScenarioAppState, TaskDTO, StepDTO};
use std::sync::atomic::Ordering;
use std::sync::{MutexGuard, PoisonError};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Mutex,
};
use tauri::State;

#[tauri::command(async)]
pub fn save_state(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = safe_get_state(state.lock());
    state.save_state();
}

#[tauri::command(async)]
pub fn get_config_path(state: State<'_, Mutex<ScenarioAppState>>) -> String {
    let state = safe_get_state(state.lock());
    state.config_path.clone()
}

#[tauri::command(async)]
pub fn get_log(state: State<'_, Mutex<ScenarioAppState>>) -> String {
    let state = safe_get_state(state.lock());
    state.output_log.clone()
}

#[tauri::command(async)]
pub fn clear_log(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = safe_get_state(state.lock());
    state.clear_log();
}

#[tauri::command(async)]
pub fn load_config(config_path: &str, state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = safe_get_state(state.lock());
    state.load_config(config_path);
}

#[tauri::command(async)]
pub fn get_required_variables(
    state: State<'_, Mutex<ScenarioAppState>>,
) -> BTreeMap<String, RequiredVariableDTO> {
    let state = safe_get_state(state.lock());
    state.get_required_variables()
}

#[tauri::command(async)]
pub fn update_required_variables(
    required_variables: HashMap<String, String>,
    state: State<'_, Mutex<ScenarioAppState>>,
) {
    let mut state = safe_get_state(state.lock());
    if let Some(scenario) = state.scenario.as_mut() {
        scenario
            .variables_mut()
            .required_mut()
            .upsert(required_variables);
    }
}

#[tauri::command(async)]
pub fn execute_scenario(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = safe_get_state(state.lock());
    if state.is_executing.load(Ordering::SeqCst) {
        eprintln!("Execution already in progress. Ignoring request.");
        return;
    }
    state.execute_scenario();
}

#[tauri::command(async)]
pub fn get_resolved_variables(
    state: State<'_, Mutex<ScenarioAppState>>,
) -> BTreeMap<String, String> {
    let mut state = safe_get_state(state.lock());
    state.get_resolved_variables()
}

#[tauri::command(async)]
pub fn clear_state(state: State<'_, Mutex<ScenarioAppState>>) -> Result<(), String> {
    let mut state = safe_get_state(state.lock());
    state.clear_state();
    Ok(())
}

#[tauri::command(async)]
pub fn get_tasks(state: State<'_, Mutex<ScenarioAppState>>) -> BTreeMap<String, TaskDTO> {
    let state = safe_get_state(state.lock());
    state.get_tasks()
}

#[tauri::command(async)]
pub fn get_steps(state: State<'_, Mutex<ScenarioAppState>>) -> Vec<StepDTO> {
    let state = safe_get_state(state.lock());
    state.get_steps()
}

fn safe_get_state<'a, T>(
    lock_result: Result<MutexGuard<'a, T>, PoisonError<MutexGuard<'a, T>>>,
) -> MutexGuard<'a, T> {
    match lock_result {
        Ok(guard) => guard,
        Err(poison_error) => {
            eprintln!(
                "WARNING: Recovered from mutex poison error: {:?}",
                poison_error
            );
            poison_error.into_inner()
        }
    }
}
