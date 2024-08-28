use crate::app::ScenarioAppState;
use std::sync::Mutex;
use tauri::State;

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
pub fn load_config(config_path: &str, state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.lock().unwrap();
    state.load_config(config_path);
}

#[tauri::command(async)]
pub fn execute_scenario(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.lock().unwrap();
    if state.is_executing {
        return;
    }
    state.execute_scenario();
}
