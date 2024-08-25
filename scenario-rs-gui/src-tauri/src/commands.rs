use crate::app::ScenarioAppState;
use std::sync::Mutex;
use tauri::State;

#[allow(dead_code)]
#[tauri::command(async)]
pub fn get_log(state: State<'_, Mutex<ScenarioAppState>>) -> String {
    let state = state.lock().unwrap();
    state.output_log.clone()
}

#[tauri::command(async)]
pub fn load_config(config_path: &str, state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.lock().unwrap();
    state.load_config(config_path);
}

#[tauri::command(async)]
pub fn execute_scenario(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.lock().unwrap();
    state.execute_scenario();
}
