use crate::{
    app::{RequiredVariableDTO, ScenarioAppState, StepDTO, TaskDTO},
    utils::SafeLock,
};
use scenario_rs::state::types::ExecutionState;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsStr,
    path::Path,
    sync::{atomic::Ordering, Mutex},
};
use tauri::State;
use tracing::warn;

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn save_state(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.safe_lock();
    state.save_state();
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn get_config_path(state: State<'_, Mutex<ScenarioAppState>>) -> String {
    let state = state.safe_lock();
    state.config_path.clone()
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn load_config(config_path: &str, state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.safe_lock();
    state.load_config(config_path);
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn get_required_variables(
    state: State<'_, Mutex<ScenarioAppState>>,
) -> BTreeMap<String, RequiredVariableDTO> {
    let state = state.safe_lock();
    state.get_required_variables()
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn update_required_variables(
    required_variables: HashMap<String, String>,
    state: State<'_, Mutex<ScenarioAppState>>,
) {
    let mut state = state.safe_lock();
    state.update_required_variables(required_variables);
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn execute_scenario(state: State<'_, Mutex<ScenarioAppState>>) {
    let mut state = state.safe_lock();
    if state.is_executing.load(Ordering::SeqCst) {
        warn!("Execution already in progress. Ignoring request.");
        return;
    }
    state.execute_scenario();
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn get_resolved_variables(
    state: State<'_, Mutex<ScenarioAppState>>,
) -> BTreeMap<String, String> {
    let mut state = state.safe_lock();
    state.get_resolved_variables()
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn clear_state(state: State<'_, Mutex<ScenarioAppState>>) -> Result<(), String> {
    let mut state = state.safe_lock();
    state.clear_state();
    Ok(())
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn get_tasks(state: State<'_, Mutex<ScenarioAppState>>) -> BTreeMap<String, TaskDTO> {
    let state = state.safe_lock();
    state.get_tasks()
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn get_steps(state: State<'_, Mutex<ScenarioAppState>>) -> Vec<StepDTO> {
    let state = state.safe_lock();
    state.get_steps()
}

#[tauri::command(async)]
pub fn is_valid_config_path(path: &str) -> bool {
    let path = Path::new(path);
    path.exists() && path.is_file() && path.extension() == Some(OsStr::new("toml"))
}

#[cfg(not(tarpaulin_include))]
#[tauri::command(async)]
pub fn get_execution_state(
    state: State<'_, Mutex<ScenarioAppState>>,
) -> Option<ExecutionState> {
    let state = state.safe_lock();
    state.get_execution_state()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_config_path_with_existing_toml() {
        // Given
        let path = "../../example_configs/example-scenario.toml";

        // When
        let result = is_valid_config_path(path);

        // Then
        assert!(result);
    }

    #[test]
    fn test_is_valid_config_path_with_nonexistent_file() {
        // Given
        let path = "nonexistent/file.toml";

        // When
        let result = is_valid_config_path(path);

        // Then
        assert!(!result);
    }

    #[test]
    fn test_is_valid_config_path_with_non_toml_extension() {
        // Given
        let path = "../../Cargo.toml";

        // When
        let result = is_valid_config_path(path);

        // Then
        assert!(result);
    }

    #[test]
    fn test_is_valid_config_path_with_directory() {
        // Given
        let path = "../../example_configs";

        // When
        let result = is_valid_config_path(path);

        // Then
        assert!(!result);
    }

    #[test]
    fn test_is_valid_config_path_with_non_toml_file() {
        // Given
        let path = "../../README.md";

        // When
        let result = is_valid_config_path(path);

        // Then
        assert!(!result);
    }
}
