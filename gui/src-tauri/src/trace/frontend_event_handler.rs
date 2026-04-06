use crate::trace::event_handler::EventHandler;
use chrono::Local;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
pub enum AppEvent {
    LogMessage(String),
    LogPlainMessage(String),
}

pub struct FrontendEventHandler;

impl EventHandler<AppEvent> for FrontendEventHandler {
    fn is_terminal(&self, _event: &AppEvent) -> bool {
        false
    }

    fn handle(&self, event: &AppEvent, app_handle: &AppHandle) {
        match event {
            AppEvent::LogMessage(message) => {
                let timestamp = Local::now().format("%H:%M:%S.%3f").to_string();
                let message = format!("[{timestamp}] {message}");
                let _ = app_handle.emit("log-message", message);
            }
            AppEvent::LogPlainMessage(text) => {
                let _ = app_handle.emit("log-message", text);
            }
        }
    }
}
