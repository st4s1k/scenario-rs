use crate::trace::{layers::EventLayer, visitors::AppEventVisitor, AppEvent};
use scenario_rs::scenario::utils::SendEvent;
use std::sync::mpsc::Sender;
use tracing::Event;

pub struct AppEventLayer {
    pub sender: Sender<AppEvent>,
}

impl AppEventLayer {
    pub fn new(sender: Sender<AppEvent>) -> Self {
        Self { sender }
    }

    pub fn send_event(&self, message: String) {
        self.sender.send_event(AppEvent::LogMessage(message));
    }
}

impl EventLayer for AppEventLayer {
    fn process_event(&self, event: &Event<'_>) {
        let mut visitor = AppEventVisitor {
            event_type: None,
            message: None,
        };

        event.record(&mut visitor);

        const APP_PREFIX: &str = "[APP] ";

        if let Some(event_type) = &visitor.event_type {
            match event_type.as_str() {
                "clear_log" => {
                    self.sender.send_event(AppEvent::ClearLog);
                    self.send_event(format!("{}Log cleared!", APP_PREFIX));
                }
                _ => {}
            }
        } else {
            if let Some(message) = &visitor.message {
                self.send_event(format!("{}{}", APP_PREFIX, message));
            }
        }
    }
}
