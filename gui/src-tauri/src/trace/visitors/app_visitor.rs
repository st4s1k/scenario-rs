use std::fmt::Debug;
use tracing::field::{Field, Visit};

pub struct AppEventVisitor {
    pub event_type: Option<String>,
    pub message: Option<String>,
}

impl Visit for AppEventVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "event" => self.event_type = Some(value.to_string()),
            "message" => self.message = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let value_str = format!("{:?}", value);
            self.message = Some(value_str.trim_matches('"').to_string());
        }
    }
}
