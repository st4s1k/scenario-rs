// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::app::ScenarioAppState;
use commands::{
    clear_log, clear_state, execute_scenario, get_config_path, get_log, get_required_variables,
    get_resolved_variables, get_steps, get_tasks, load_config, save_state,
    update_required_variables,
};
use utils::SafeLock;
use std::sync::Mutex;
use tauri::Manager;

mod app;
mod app_event;
mod commands;
mod event;
mod scenario_event;
mod shared;
mod utils;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_state = ScenarioAppState::new(app.handle());
            app.manage(Mutex::new(app_state));
            let state = app.handle().state::<Mutex<ScenarioAppState>>();
            let mut app_state = state.safe_lock();
            app_state.init();
            Ok(())
        })
        .on_window_event(on_window_event)
        .invoke_handler(tauri::generate_handler![
            save_state,
            get_config_path,
            get_log,
            clear_log,
            load_config,
            get_required_variables,
            update_required_variables,
            get_resolved_variables,
            execute_scenario,
            clear_state,
            get_tasks,
            get_steps
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::Destroyed { .. } = event {
        let app_handle = window.app_handle();
        let state = app_handle.state::<Mutex<ScenarioAppState>>();
        let mut state = state.lock().unwrap();
        state.save_state();
    }
}
