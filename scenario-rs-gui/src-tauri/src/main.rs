// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::app::ScenarioAppState;
use commands::{get_log, execute_scenario, load_config};
use std::sync::Mutex;
use tauri::Manager;

mod app;
mod commands;
mod lifecycle;
mod shared;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let mut state = ScenarioAppState::new(app.app_handle());
            state.load_state();
            app.manage(Mutex::new(state));
            Ok(())
        })
        .on_window_event(on_window_event)
        .invoke_handler(tauri::generate_handler![get_log, load_config, execute_scenario])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn on_window_event(event: tauri::GlobalWindowEvent) {
    if let tauri::WindowEvent::CloseRequested { .. } = event.event() {
        let app_handle = event.window().app_handle();
        let state = app_handle.state::<Mutex<ScenarioAppState>>();
        let mut state = state.lock().unwrap();
        state.save_state();
    }
}
