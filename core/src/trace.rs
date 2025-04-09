use std::fmt;
use tracing::field::{Field, Visit};

/// A visitor struct for tracing events in scenarios.
///
/// This struct collects event fields from tracing spans and events,
/// providing structured access to various event properties such as
/// event type, descriptions, progress information, and error details.
pub struct ScenarioEventVisitor {
    /// Type of the scenario event (e.g., "step_started", "task_completed")
    pub event_type: Option<String>,
    /// Human-readable description of the event
    pub description: Option<String>,
    /// Current step/task index in a sequence
    pub index: Option<usize>,
    /// Total number of steps in the current operation
    pub total_steps: Option<usize>,
    /// Command being executed, if applicable
    pub command: Option<String>,
    /// Output from command execution
    pub output: Option<String>,
    /// Error message if an error occurred
    pub error: Option<String>,
    /// Source path for file transfer operations
    pub source: Option<String>,
    /// Destination path for file transfer operations
    pub destination: Option<String>,
    /// Current progress value (e.g., bytes transferred)
    pub current: Option<u64>,
    /// Total expected progress value
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
