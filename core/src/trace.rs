use std::fmt;
use tracing::field::{Field, Visit};

pub struct ScenarioEventVisitor {
    pub event_type: Option<String>,
    pub description: Option<String>,
    pub index: Option<usize>,
    pub total_steps: Option<usize>,
    pub command: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub current: Option<u64>,
    pub total: Option<u64>,
}

impl Visit for ScenarioEventVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "event" => self.event_type = Some(value.to_string()),
            "description" => self.description = Some(value.to_string()),
            "command" => self.command = Some(value.to_string()),
            "output" => self.output = Some(value.to_string()),
            "error" => self.error = Some(value.to_string()),
            "source" => self.source = Some(value.to_string()),
            "destination" => self.destination = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value_str = format!("{:?}", value);
        match field.name() {
            "index" => self.index = value_str.parse().ok(),
            "total_steps" => self.total_steps = value_str.parse().ok(),
            "current" => self.current = value_str.parse().ok(),
            "total" => self.total = value_str.parse().ok(),
            "error" => self.error = Some(value_str),
            "event" => self.event_type = Some(value_str),
            "description" => self.description = Some(value_str),
            "command" => self.command = Some(value_str),
            "output" => self.output = Some(value_str),
            "source" => self.source = Some(value_str),
            "destination" => self.destination = Some(value_str),
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        match field.name() {
            "index" => self.index = Some(value as usize),
            "total_steps" => self.total_steps = Some(value as usize),
            "current" => self.current = Some(value),
            "total" => self.total = Some(value),
            _ => {}
        }
    }
}
