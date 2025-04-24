use std::fmt;
use tracing::{
    error,
    field::{Field, Visit},
};

/// A visitor struct for tracing events in scenarios.
///
/// This struct collects event fields from tracing spans and events,
/// providing structured access to various event properties such as
/// event type, descriptions, progress information, and error details.
///
/// # Examples
///
/// Creating a new visitor and collecting event fields:
///
/// ```
/// use scenario_rs_core::trace::ScenarioEventVisitor;
/// use tracing::field::{Field, Visit};
///
/// fn field(name: &str) -> Field {
///     struct TestCallsite();
///     impl tracing::callsite::Callsite for TestCallsite {
///         fn set_interest(&self, _: tracing::subscriber::Interest) {
///             unimplemented!()
///         }
///
///         fn metadata(&self) -> &tracing::Metadata<'_> {
///             &TEST_META
///         }
///     }
///     static TEST_CALLSITE: TestCallsite = TestCallsite();
///     static TEST_META: tracing::Metadata<'static> = tracing::metadata! {
///         name: "field_test",
///         target: module_path!(),
///         level: tracing::metadata::Level::INFO,
///         fields: &[
///              "event",
///             "description",
///             "command",
///             "index",
///             "total_steps",
///         ],
///         callsite: &TEST_CALLSITE,
///         kind: tracing::metadata::Kind::SPAN,
///     };
///
///     tracing::field::AsField::as_field(name, &TEST_META).unwrap()
/// }
///
/// // Create a new visitor
/// let mut visitor = ScenarioEventVisitor::default();
///
/// // Record string fields
/// visitor.record_str(&field("event"), "step_started");
/// visitor.record_str(&field("description"), "Installing dependencies");
/// visitor.record_str(&field("command"), "apt-get install -y nginx");
///
/// // Record numeric fields
/// visitor.record_u64(&field("index"), 1);
/// visitor.record_u64(&field("total_steps"), 5);
///
/// // Access the collected fields
/// assert_eq!(visitor.event_type.unwrap(), "step_started");
/// assert_eq!(visitor.description.unwrap(), "Installing dependencies");
/// assert_eq!(visitor.index.unwrap(), 1);
/// assert_eq!(visitor.total_steps.unwrap(), 5);
/// ```
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

impl ScenarioEventVisitor {
    /// Creates a new empty visitor.
    ///
    /// All fields are initialized to `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use scenario_rs_core::trace::ScenarioEventVisitor;
    ///
    /// let visitor = ScenarioEventVisitor::new();
    /// assert!(visitor.event_type.is_none());
    /// assert!(visitor.description.is_none());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ScenarioEventVisitor {
    fn default() -> Self {
        ScenarioEventVisitor {
            event_type: None,
            description: None,
            index: None,
            total_steps: None,
            command: None,
            output: None,
            error: None,
            source: None,
            destination: None,
            current: None,
            total: None,
        }
    }
}

impl Visit for ScenarioEventVisitor {
    /// Records string values from tracing events.
    ///
    /// This method processes string fields from tracing events and stores them
    /// in the appropriate field based on the field name.
    ///
    /// # Arguments
    ///
    /// * `field` - The field metadata containing the field name
    /// * `value` - The string value to record
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "event" => self.event_type = Some(value.to_string()),
            "description" => self.description = Some(value.to_string()),
            "command" => self.command = Some(value.to_string()),
            "output" => self.output = Some(value.to_string()),
            "error" => self.error = Some(value.to_string()),
            "source" => self.source = Some(value.to_string()),
            "destination" => self.destination = Some(value.to_string()),
            "message" => {}
            "host" => {}
            "username" => {}
            "password" => {}
            _ => {
                error!("Unrecognized field: {}", field.name());
            }
        }
    }

    /// Records debug-formatted values from tracing events.
    ///
    /// This method processes fields that implement `Debug` and attempts to
    /// convert and store them in the appropriate field based on the field name.
    ///
    /// # Arguments
    ///
    /// * `field` - The field metadata containing the field name
    /// * `value` - The debug-formattable value to record
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value_str = format!("{:?}", value).trim_matches('"').to_string();
        match field.name() {
            "error" => self.error = Some(value_str),
            "event" => self.event_type = Some(value_str),
            "description" => self.description = Some(value_str),
            "command" => self.command = Some(value_str),
            "output" => self.output = Some(value_str),
            "source" => self.source = Some(value_str),
            "destination" => self.destination = Some(value_str),
            "message" => {}
            "host" => {}
            "username" => {}
            "password" => {}
            _ => {
                error!("Unrecognized field: {}", field.name());
            }
        }
    }

    /// Records unsigned 64-bit integer values from tracing events.
    ///
    /// This method processes numeric fields and stores them in the appropriate
    /// field based on the field name.
    ///
    /// # Arguments
    ///
    /// * `field` - The field metadata containing the field name
    /// * `value` - The u64 value to record
    fn record_u64(&mut self, field: &Field, value: u64) {
        match field.name() {
            "index" => self.index = Some(value as usize),
            "total_steps" => self.total_steps = Some(value as usize),
            "current" => self.current = Some(value),
            "total" => self.total = Some(value),
            "port" => {}
            _ => {
                error!("Unrecognized field: {}", field.name());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::trace::ScenarioEventVisitor;
    use tracing::field::{Field, Visit};

    #[test]
    fn test_visitor_new_creates_empty_visitor() {
        // Given & When
        let visitor = ScenarioEventVisitor::new();

        // Then
        assert!(visitor.event_type.is_none());
        assert!(visitor.description.is_none());
        assert!(visitor.index.is_none());
        assert!(visitor.total_steps.is_none());
        assert!(visitor.command.is_none());
        assert!(visitor.output.is_none());
        assert!(visitor.error.is_none());
        assert!(visitor.source.is_none());
        assert!(visitor.destination.is_none());
        assert!(visitor.current.is_none());
        assert!(visitor.total.is_none());
    }

    #[test]
    fn test_visitor_record_str() {
        // Given
        let mut visitor = ScenarioEventVisitor::default();

        // When
        visitor.record_str(&field("event"), "step_started");
        visitor.record_str(&field("description"), "Installing dependencies");
        visitor.record_str(&field("command"), "apt-get update");
        visitor.record_str(&field("output"), "Reading package lists...");
        visitor.record_str(&field("error"), "Connection failed");
        visitor.record_str(&field("source"), "/local/file.txt");
        visitor.record_str(&field("destination"), "/remote/file.txt");
        visitor.record_str(&field("ignored_field"), "Should be ignored");

        // Then
        assert_eq!(visitor.event_type.unwrap(), "step_started");
        assert_eq!(visitor.description.unwrap(), "Installing dependencies");
        assert_eq!(visitor.command.unwrap(), "apt-get update");
        assert_eq!(visitor.output.unwrap(), "Reading package lists...");
        assert_eq!(visitor.error.unwrap(), "Connection failed");
        assert_eq!(visitor.source.unwrap(), "/local/file.txt");
        assert_eq!(visitor.destination.unwrap(), "/remote/file.txt");
    }

    #[test]
    fn test_visitor_record_u64() {
        // Given
        let mut visitor = ScenarioEventVisitor::default();

        // When
        visitor.record_u64(&field("index"), 2);
        visitor.record_u64(&field("total_steps"), 5);
        visitor.record_u64(&field("current"), 1024);
        visitor.record_u64(&field("total"), 4096);
        visitor.record_u64(&field("ignored_field"), 42);

        // Then
        assert_eq!(visitor.index.unwrap(), 2);
        assert_eq!(visitor.total_steps.unwrap(), 5);
        assert_eq!(visitor.current.unwrap(), 1024);
        assert_eq!(visitor.total.unwrap(), 4096);
    }

    #[test]
    fn test_visitor_record_debug() {
        // Given
        let mut visitor = ScenarioEventVisitor::default();

        // When
        visitor.record_debug(&field("event"), &"step_started");
        visitor.record_debug(&field("description"), &"Installing dependencies");
        visitor.record_debug(&field("command"), &"apt-get update");
        visitor.record_debug(&field("output"), &"Reading package lists...");
        visitor.record_debug(&field("error"), &"Connection failed");
        visitor.record_debug(&field("source"), &"/local/file.txt");
        visitor.record_debug(&field("destination"), &"/remote/file.txt");
        visitor.record_debug(&field("ignored_field"), &"Should be ignored");

        // Then
        assert_eq!(visitor.event_type.unwrap(), "step_started");
        assert_eq!(visitor.description.unwrap(), "Installing dependencies");
        assert_eq!(visitor.command.unwrap(), "apt-get update");
        assert_eq!(visitor.output.unwrap(), "Reading package lists...");
        assert_eq!(visitor.error.unwrap(), "Connection failed");
        assert_eq!(visitor.source.unwrap(), "/local/file.txt");
        assert_eq!(visitor.destination.unwrap(), "/remote/file.txt");
    }

    #[test]
    fn test_visitor_record_debug_invalid_parse() {
        // Given
        let mut visitor = ScenarioEventVisitor::default();

        // When
        visitor.record_debug(&field("index"), &"not_a_number");

        // Then
        assert!(visitor.index.is_none());
    }

    #[test]
    fn test_visitor_default() {
        // Given & When
        let visitor = ScenarioEventVisitor::default();

        // Then
        assert!(visitor.event_type.is_none());
        assert!(visitor.description.is_none());
        assert!(visitor.index.is_none());
    }

    // Test helpers
    fn field(name: &str) -> Field {
        struct TestCallsite();
        impl tracing::callsite::Callsite for TestCallsite {
            fn set_interest(&self, _: tracing::subscriber::Interest) {
                unimplemented!()
            }

            fn metadata(&self) -> &tracing::Metadata<'_> {
                &TEST_META
            }
        }
        static TEST_CALLSITE: TestCallsite = TestCallsite();
        static TEST_META: tracing::Metadata<'static> = tracing::metadata! {
            name: "field_test",
            target: module_path!(),
            level: tracing::metadata::Level::INFO,
            fields: &[
                    "event",
                    "description",
                    "command",
                    "index",
                    "total_steps",
                    "output",
                    "error",
                    "source",
                    "destination",
                    "ignored_field",
                    "current",
                    "total",
            ],
            callsite: &TEST_CALLSITE,
            kind: tracing::metadata::Kind::SPAN,
        };

        tracing::field::AsField::as_field(name, &TEST_META).unwrap()
    }
}
