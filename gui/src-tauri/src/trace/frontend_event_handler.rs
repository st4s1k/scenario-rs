use crate::trace::event_handler::EventHandler;
use chrono::Local;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum StepState {
    SftpCopyProgress {
        source: String,
        destination: String,
        current: u64,
        total: u64,
    },
    RemoteSudoOutput {
        command: String,
        output: String,
    },
    StepCompleted {
        index: usize,
    },
    StepFailed {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Execution(bool),
    LogMessage(String),
    StepState { state: StepState },
    StepIndex { index: usize },
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
            AppEvent::LogMessage(message) => {
                let timestamp = Local::now().format("%H:%M:%S.%3f").to_string();
                let message = format!("[{timestamp}] {message}");
                let _ = app_handle.emit("log-message", message);
            }
            AppEvent::StepState { state } => {
                let _ = app_handle.emit("step-state", state);
            }
            AppEvent::StepIndex { index } => {
                let _ = app_handle.emit("step-index", index);
            }
        }
    }
}
