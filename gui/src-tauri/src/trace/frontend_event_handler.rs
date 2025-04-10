use crate::{app::ScenarioAppState, trace::event_handler::EventHandler, utils::SafeLock};
use chrono::Local;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone)]
pub enum AppEvent {
    Execution(bool),
    ClearLog,
    LogMessage(String),
}

pub struct FrontendEventHandler;

impl EventHandler<AppEvent> for FrontendEventHandler {
    fn is_terminal(&self, _event: &AppEvent) -> bool {
        false
    }

    fn handle(&self, event: &AppEvent, app_handle: &AppHandle) {
        match event {
            AppEvent::Execution(is_starting) => {
                let _ = app_handle.emit("execution-status", *is_starting);
            }
            AppEvent::ClearLog => {
                let state = app_handle.state::<Mutex<ScenarioAppState>>();
                let mut state = state.safe_lock();
                state.output_log.clear();
                let _ = app_handle.emit("log-update", ());
            }
            AppEvent::LogMessage(message) => {
                let state = app_handle.state::<Mutex<ScenarioAppState>>();
                let mut state = state.safe_lock();
                let timestamp = Local::now().format("%H:%M:%S.%3f").to_string();
                state
                    .output_log
                    .push_str(&format!("[{timestamp}] {message}\n"));
                let _ = app_handle.emit("log-update", ());
            }
        }
    }
}
