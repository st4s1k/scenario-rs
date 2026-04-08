// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::{
    app::ScenarioAppState,
    commands::{
        clear_state, execute_scenario, get_config_path, get_debug_mode, get_execution_state,
        get_required_variables, get_resolved_variables, get_steps, get_tasks,
        is_valid_config_path, load_config, save_state, set_debug_mode, update_required_variables,
    },
    trace::{AppEvent, FrontendLayer},
    utils::SafeLock,
};
use std::sync::Mutex;
use tauri::Manager;
use tracing::Level;
use tracing_subscriber::{
    filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

mod app;
mod commands;
mod trace;
mod utils;

fn main() {
    let (frontend_tx, frontend_rx) = std::sync::mpsc::channel::<AppEvent>();

    let env_filter = EnvFilter::builder()
        .with_default_directive(
            if cfg!(debug_assertions) {
                "trace"
            } else {
                "info"
            }
            .parse()
            .unwrap(),
        )
        .from_env_lossy()
        .add_directive("russh=info".parse().unwrap())
        .add_directive("russh_sftp=info".parse().unwrap());

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .compact()
                .with_target(false)
                .with_filter(env_filter),
        )
        .with(FrontendLayer::from(frontend_tx).with_filter(
            EnvFilter::new("trace")
                .add_directive("russh=info".parse().unwrap())
                .add_directive("russh_sftp=info".parse().unwrap()),
        ))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_prevent_default::debug())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_state = ScenarioAppState::new(app.handle());
            app.manage(Mutex::new(app_state));
            let state = app.handle().state::<Mutex<ScenarioAppState>>();
            let mut app_state = state.safe_lock();
            app_state.init(frontend_rx);
            Ok(())
        })
        .on_window_event(on_window_event)
        .invoke_handler(tauri::generate_handler![
            save_state,
            get_config_path,
            load_config,
            get_required_variables,
            update_required_variables,
            get_resolved_variables,
            execute_scenario,
            clear_state,
            get_tasks,
            get_steps,
            is_valid_config_path,
            get_execution_state,
            get_debug_mode,
            set_debug_mode
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
