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
///             "scenario.event",
///             "task.description",
///             "remote_sudo.command",
///             "step.index",
///             "steps.total",
///             "on_fail_step.index",
///             "on_fail_steps.total",
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
/// visitor.record_str(&field("scenario.event"), "step_started");
/// visitor.record_str(&field("task.description"), "Installing dependencies");
/// visitor.record_str(&field("remote_sudo.command"), "apt-get install -y nginx");
///
/// // Record numeric fields
/// visitor.record_u64(&field("step.index"), 1);
/// visitor.record_u64(&field("steps.total"), 5);
///
/// // Access the collected fields
/// assert_eq!(visitor.scenario_event.unwrap(), "step_started");
/// assert_eq!(visitor.task_description.unwrap(), "Installing dependencies");
/// assert_eq!(visitor.step_index.unwrap(), 1);
/// assert_eq!(visitor.steps_total.unwrap(), 5);
/// ```
#[derive(Debug, Clone)]
pub struct ScenarioEventVisitor {
    /// Type of the scenario event (e.g., "step_started", "task_completed")
    pub scenario_event: Option<String>,
    /// Error message if an error occurred
    pub scenario_error: Option<String>,
    /// Human-readable description of the event
    pub task_description: Option<String>,
    /// Command being executed, if applicable
    pub remote_sudo_command: Option<String>,
    /// Output from command execution
    pub remote_sudo_output: Option<String>,
    /// Exit status of the command, if applicable
    pub remote_sudo_exit_status: Option<i64>,
    /// Source path for file transfer operations
    pub sftp_copy_source: Option<String>,
    /// Destination path for file transfer operations
    pub sftp_copy_destination: Option<String>,
    /// Current progress value (e.g., bytes transferred)
    pub sftp_copy_progress_current: Option<u64>,
    /// Total expected progress value
    pub sftp_copy_progress_total: Option<u64>,
    /// Current step/task index in a sequence
    pub step_index: Option<usize>,
    /// Total number of steps in the current operation
    pub steps_total: Option<usize>,
    /// Current on-fail step index in a sequence
    pub on_fail_step_index: Option<usize>,
    /// Total number of on-fail steps
    pub on_fail_steps_total: Option<usize>,
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
    /// assert!(visitor.scenario_event.is_none());
    /// assert!(visitor.task_description.is_none());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: &Self) {
        macro_rules! fill {
            ($field:ident) => {
                if self.$field.is_none() {
                    self.$field = other.$field.clone();
                }
            };
        }
        fill!(scenario_event);
        fill!(scenario_error);
        fill!(task_description);
        fill!(remote_sudo_command);
        fill!(remote_sudo_output);
        fill!(remote_sudo_exit_status);
        fill!(sftp_copy_source);
        fill!(sftp_copy_destination);
        fill!(sftp_copy_progress_current);
        fill!(sftp_copy_progress_total);
        fill!(step_index);
        fill!(steps_total);
        fill!(on_fail_step_index);
        fill!(on_fail_steps_total);
    }
}

impl Default for ScenarioEventVisitor {
    fn default() -> Self {
        ScenarioEventVisitor {
            scenario_event: None,
            scenario_error: None,
            task_description: None,
            remote_sudo_command: None,
            remote_sudo_output: None,
            remote_sudo_exit_status: None,
            sftp_copy_source: None,
            sftp_copy_destination: None,
            sftp_copy_progress_current: None,
            sftp_copy_progress_total: None,
            step_index: None,
            steps_total: None,
            on_fail_step_index: None,
            on_fail_steps_total: None,
        }
    }
}

impl Visit for ScenarioEventVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => {}
            "session.host" => {}
            "session.username" => {}
            "session.password" => {}
            "scenario.event" => self.scenario_event = Some(value.to_string()),
            "scenario.error" => self.scenario_error = Some(value.to_string()),
            "task.description" => self.task_description = Some(value.to_string()),
            "remote_sudo.command" => self.remote_sudo_command = Some(value.to_string()),
            "remote_sudo.output" => self.remote_sudo_output = Some(value.to_string()),
            "sftp_copy.source" => self.sftp_copy_source = Some(value.to_string()),
            "sftp_copy.destination" => self.sftp_copy_destination = Some(value.to_string()),
            _ => {
                error!("Unrecognized field: {}", field.name());
            }
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value_str = format!("{:?}", value).trim_matches('"').to_string();
        match field.name() {
            "message" => {}
            "session.host" => {}
            "session.username" => {}
            "session.password" => {}
            "scenario.event" => self.scenario_event = Some(value_str),
            "scenario.error" => self.scenario_error = Some(value_str),
            "task.description" => self.task_description = Some(value_str),
            "remote_sudo.command" => self.remote_sudo_command = Some(value_str),
            "remote_sudo.output" => self.remote_sudo_output = Some(value_str),
            "sftp_copy.source" => self.sftp_copy_source = Some(value_str),
            "sftp_copy.destination" => self.sftp_copy_destination = Some(value_str),
            _ => {
                error!("Unrecognized field: {}", field.name());
            }
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        match field.name() {
            "session.port" => {}
            "sftp_copy.progress.current" => self.sftp_copy_progress_current = Some(value),
            "sftp_copy.progress.total" => self.sftp_copy_progress_total = Some(value),
            "step.index" => self.step_index = Some(value as usize),
            "steps.total" => self.steps_total = Some(value as usize),
            "on_fail_step.index" => self.on_fail_step_index = Some(value as usize),
            "on_fail_steps.total" => self.on_fail_steps_total = Some(value as usize),
            _ => {
                error!("Unrecognized field: {}", field.name());
            }
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        match field.name() {
            "remote_sudo.exit_status" => {
                self.remote_sudo_exit_status = Some(value);
            }
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
        assert!(visitor.scenario_event.is_none());
        assert!(visitor.scenario_error.is_none());
        assert!(visitor.task_description.is_none());
        assert!(visitor.remote_sudo_command.is_none());
        assert!(visitor.remote_sudo_output.is_none());
        assert!(visitor.remote_sudo_exit_status.is_none());
        assert!(visitor.sftp_copy_source.is_none());
        assert!(visitor.sftp_copy_destination.is_none());
        assert!(visitor.sftp_copy_progress_current.is_none());
        assert!(visitor.sftp_copy_progress_total.is_none());
        assert!(visitor.step_index.is_none());
        assert!(visitor.steps_total.is_none());
        assert!(visitor.on_fail_step_index.is_none());
        assert!(visitor.on_fail_steps_total.is_none());
    }

    #[test]
    fn test_visitor_record_str() {
        // Given
        let mut visitor = ScenarioEventVisitor::default();

        // When
        visitor.record_str(&field("scenario.event"), "step_started");
        visitor.record_str(&field("scenario.error"), "Connection failed");
        visitor.record_str(&field("task.description"), "Installing dependencies");
        visitor.record_str(&field("remote_sudo.command"), "apt-get update");
        visitor.record_str(&field("remote_sudo.output"), "Reading package lists...");
        visitor.record_str(&field("sftp_copy.source"), "/local/file.txt");
        visitor.record_str(&field("sftp_copy.destination"), "/remote/file.txt");
        visitor.record_str(&field("ignored_field"), "Should be ignored");

        // Then
        assert_eq!(visitor.scenario_event.unwrap(), "step_started");
        assert_eq!(visitor.task_description.unwrap(), "Installing dependencies");
        assert_eq!(visitor.remote_sudo_command.unwrap(), "apt-get update");
        assert_eq!(
            visitor.remote_sudo_output.unwrap(),
            "Reading package lists..."
        );
        assert_eq!(visitor.scenario_error.unwrap(), "Connection failed");
        assert_eq!(visitor.sftp_copy_source.unwrap(), "/local/file.txt");
        assert_eq!(visitor.sftp_copy_destination.unwrap(), "/remote/file.txt");
    }

    #[test]
    fn test_visitor_record_u64() {
        // Given
        let mut visitor = ScenarioEventVisitor::default();

        // When
        visitor.record_u64(&field("sftp_copy.progress.current"), 1024);
        visitor.record_u64(&field("sftp_copy.progress.total"), 4096);
        visitor.record_u64(&field("step.index"), 2);
        visitor.record_u64(&field("steps.total"), 5);
        visitor.record_u64(&field("on_fail_step.index"), 1);
        visitor.record_u64(&field("on_fail_steps.total"), 3);
        visitor.record_u64(&field("ignored_field"), 42);

        // Then
        assert_eq!(visitor.sftp_copy_progress_current.unwrap(), 1024);
        assert_eq!(visitor.sftp_copy_progress_total.unwrap(), 4096);
        assert_eq!(visitor.step_index.unwrap(), 2);
        assert_eq!(visitor.steps_total.unwrap(), 5);
        assert_eq!(visitor.on_fail_step_index.unwrap(), 1);
        assert_eq!(visitor.on_fail_steps_total.unwrap(), 3);
    }

    #[test]
    fn test_visitor_record_i64() {
        // Given
        let mut visitor = ScenarioEventVisitor::default();

        // When
        visitor.record_i64(&field("remote_sudo.exit_status"), 0);
        visitor.record_i64(&field("ignored_field"), -1);

        // Then
        assert_eq!(visitor.remote_sudo_exit_status.unwrap(), 0);
    }

    #[test]
    fn test_visitor_record_debug() {
        // Given
        let mut visitor = ScenarioEventVisitor::default();

        // When
        visitor.record_debug(&field("scenario.event"), &"step_started");
        visitor.record_debug(&field("scenario.error"), &"Connection failed");
        visitor.record_debug(&field("task.description"), &"Installing dependencies");
        visitor.record_debug(&field("remote_sudo.command"), &"apt-get update");
        visitor.record_debug(&field("remote_sudo.output"), &"Reading package lists...");
        visitor.record_debug(&field("sftp_copy.source"), &"/local/file.txt");
        visitor.record_debug(&field("sftp_copy.destination"), &"/remote/file.txt");
        visitor.record_debug(&field("ignored_field"), &"Should be ignored");

        // Then
        assert_eq!(visitor.scenario_event.unwrap(), "step_started");
        assert_eq!(visitor.scenario_error.unwrap(), "Connection failed");
        assert_eq!(visitor.task_description.unwrap(), "Installing dependencies");
        assert_eq!(visitor.remote_sudo_command.unwrap(), "apt-get update");
        assert_eq!(
            visitor.remote_sudo_output.unwrap(),
            "Reading package lists..."
        );
        assert_eq!(visitor.sftp_copy_source.unwrap(), "/local/file.txt");
        assert_eq!(visitor.sftp_copy_destination.unwrap(), "/remote/file.txt");
    }

    #[test]
    fn test_visitor_record_debug_invalid_parse() {
        // Given
        let mut visitor = ScenarioEventVisitor::default();

        // When
        visitor.record_debug(&field("step.index"), &"not_a_number");

        // Then
        assert!(visitor.step_index.is_none());
    }

    #[test]
    fn test_visitor_default() {
        // Given & When
        let visitor = ScenarioEventVisitor::default();

        // Then
        assert!(visitor.scenario_event.is_none());
        assert!(visitor.scenario_error.is_none());
        assert!(visitor.task_description.is_none());
        assert!(visitor.remote_sudo_command.is_none());
        assert!(visitor.remote_sudo_output.is_none());
        assert!(visitor.remote_sudo_exit_status.is_none());
        assert!(visitor.sftp_copy_source.is_none());
        assert!(visitor.sftp_copy_destination.is_none());
        assert!(visitor.sftp_copy_progress_current.is_none());
        assert!(visitor.sftp_copy_progress_total.is_none());
        assert!(visitor.step_index.is_none());
        assert!(visitor.steps_total.is_none());
        assert!(visitor.on_fail_step_index.is_none());
        assert!(visitor.on_fail_steps_total.is_none());
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
                    "scenario.event",
                    "scenario.error",
                    "task.description",

                    "remote_sudo.command",
                    "remote_sudo.output",
                    "remote_sudo.exit_status",

                    "sftp_copy.source",
                    "sftp_copy.destination",
                    "sftp_copy.progress.current",
                    "sftp_copy.progress.total",

                    "step.index",
                    "steps.total",

                    "on_fail_step.index",
                    "on_fail_steps.total",

                    "ignored_field",
            ],
            callsite: &TEST_CALLSITE,
            kind: tracing::metadata::Kind::SPAN,
        };

        tracing::field::AsField::as_field(name, &TEST_META).unwrap()
    }
}
