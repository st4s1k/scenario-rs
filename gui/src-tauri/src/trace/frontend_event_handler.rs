use crate::trace::event_handler::EventHandler;
use chrono::Local;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
pub enum AppEvent {
    LogMessage(String),
    LogPlainMessage(String),
    LogProgress(String),
}

pub struct FrontendEventHandler;

#[cfg(not(tarpaulin_include))]
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
            AppEvent::LogProgress(message) => {
                let timestamp = Local::now().format("%H:%M:%S.%3f").to_string();
                let message = format!("[{timestamp}] {message}");
                let _ = app_handle.emit("log-progress", message);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::event_handler::EventHandler;

    #[test]
    fn test_is_terminal_returns_false_for_log_message() {
        // Given
        let handler = FrontendEventHandler;
        let event = AppEvent::LogMessage("test".to_string());

        // When
        let result = handler.is_terminal(&event);

        // Then
        assert!(!result);
    }

    #[test]
    fn test_is_terminal_returns_false_for_log_plain_message() {
        // Given
        let handler = FrontendEventHandler;
        let event = AppEvent::LogPlainMessage("plain".to_string());

        // When
        let result = handler.is_terminal(&event);

        // Then
        assert!(!result);
    }

    #[test]
    fn test_is_terminal_returns_false_for_log_progress() {
        // Given
        let handler = FrontendEventHandler;
        let event = AppEvent::LogProgress("progress".to_string());

        // When
        let result = handler.is_terminal(&event);

        // Then
        assert!(!result);
    }

    #[test]
    fn test_app_event_log_message_holds_string() {
        // Given & When
        let event = AppEvent::LogMessage("hello world".to_string());

        // Then
        match event {
            AppEvent::LogMessage(msg) => assert_eq!(msg, "hello world"),
            _ => panic!("Expected LogMessage variant"),
        }
    }

    #[test]
    fn test_app_event_log_plain_message_holds_string() {
        // Given & When
        let event = AppEvent::LogPlainMessage("raw output".to_string());

        // Then
        match event {
            AppEvent::LogPlainMessage(msg) => assert_eq!(msg, "raw output"),
            _ => panic!("Expected LogPlainMessage variant"),
        }
    }

    #[test]
    fn test_app_event_log_progress_holds_string() {
        // Given & When
        let event = AppEvent::LogProgress("50% done".to_string());

        // Then
        match event {
            AppEvent::LogProgress(msg) => assert_eq!(msg, "50% done"),
            _ => panic!("Expected LogProgress variant"),
        }
    }

    #[test]
    fn test_app_event_clone() {
        // Given
        let original = AppEvent::LogMessage("clone me".to_string());

        // When
        let cloned = original.clone();

        // Then
        match cloned {
            AppEvent::LogMessage(msg) => assert_eq!(msg, "clone me"),
            _ => panic!("Expected LogMessage variant"),
        }
    }
}
