use crate::{
    trace::{layers::EventLayer, AppEvent},
    utils::SendEvent,
};
use scenario_rs::trace::{ScenarioEvent, ScenarioEventVisitor};
use std::sync::mpsc::Sender;
use tracing::span::Record;
use tracing::{error, span::Attributes, Event, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan};

pub struct ScenarioEventLayer {
    pub sender: Sender<AppEvent>,
}

impl ScenarioEventLayer {
    pub fn new(sender: Sender<AppEvent>) -> Self {
        Self { sender }
    }
}

impl EventLayer for ScenarioEventLayer {
    fn on_new_span<S>(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let mut visitor = ScenarioEventVisitor::default();
        attrs.record(&mut visitor);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(visitor);
        }
    }

    fn on_record<S>(&self, id: &Id, record: &Record<'_>, ctx: Context<'_, S>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        if let Some(span) = ctx.span(id) {
            if let Some(v) = span.extensions_mut().get_mut::<ScenarioEventVisitor>() {
                record.record(v);
            }
        }
    }

    fn process_event<S>(&self, event: &Event<'_>, ctx: Context<'_, S>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let mut visitor = ScenarioEventVisitor::default();

        event.record(&mut visitor);

        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                if let Some(extra) = span.extensions().get::<ScenarioEventVisitor>() {
                    visitor.merge(extra);
                }
            }
        }

        const SCENARIO_PREFIX: &str = "[SCN]";

        if let Some(scenario_event_str) = visitor.scenario_event {
            let Ok(scenario_event) = scenario_event_str.parse::<ScenarioEvent>() else {
                error!("Unrecognized event type: {}", scenario_event_str);
                return;
            };

            // Only emit log messages — progress/state is handled by ExecutionStateManager
            match scenario_event {
                ScenarioEvent::Error => {
                    if let Some(scenario_error) = visitor.scenario_error {
                        if let (Some(step_index), Some(steps_total)) =
                            (visitor.step_index, visitor.steps_total)
                        {
                            if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                                (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                            {
                                self.sender.send_event(AppEvent::LogMessage(format!(
                                    "{} [{}/{}] [on-fail] [{}/{}] {}",
                                    SCENARIO_PREFIX,
                                    step_index + 1,
                                    steps_total,
                                    on_fail_step_index + 1,
                                    on_fail_steps_total,
                                    scenario_error
                                )));
                            } else {
                                self.sender.send_event(AppEvent::LogMessage(format!(
                                    "{} [{}/{}] {}",
                                    SCENARIO_PREFIX,
                                    step_index + 1,
                                    steps_total,
                                    scenario_error
                                )));
                            }
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} {}",
                                SCENARIO_PREFIX, scenario_error
                            )));
                        }
                    } else {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} Scenario execution failed",
                            SCENARIO_PREFIX
                        )));
                    }
                }
                ScenarioEvent::ScenarioStarted => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{} Scenario started...",
                        SCENARIO_PREFIX
                    )));
                }
                ScenarioEvent::ScenarioCompleted => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{} Scenario completed successfully!",
                        SCENARIO_PREFIX
                    )));
                }
                ScenarioEvent::ScenarioFailed => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{} Scenario failed",
                        SCENARIO_PREFIX
                    )));
                }
                ScenarioEvent::StepStarted => {
                    if let (Some(step_index), Some(steps_total), Some(task_description)) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.task_description,
                    ) {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} [{}/{}] {}",
                            SCENARIO_PREFIX,
                            step_index + 1,
                            steps_total,
                            task_description
                        )));
                    }
                }
                ScenarioEvent::RemoteSudoStarted => {
                    if let (Some(step_index), Some(steps_total), Some(remote_sudo_command)) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.remote_sudo_command,
                    ) {
                        if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                            (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                        {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] Command: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total,
                                remote_sudo_command
                            )));
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] Command: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                remote_sudo_command
                            )));
                        }
                    }
                }
                ScenarioEvent::RemoteSudoOutput => {
                    if let (Some(step_index), Some(steps_total), Some(remote_sudo_output)) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.remote_sudo_output,
                    ) {
                        if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                            (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                        {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] Output:",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total
                            )));
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] Output:",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total
                            )));
                        }
                        self.sender
                            .send_event(AppEvent::LogPlainMessage(remote_sudo_output));
                    }
                }
                ScenarioEvent::SftpCopyStarted => {
                    if let (
                        Some(step_index),
                        Some(steps_total),
                        Some(sftp_copy_source),
                        Some(sftp_copy_destination),
                    ) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.sftp_copy_source,
                        visitor.sftp_copy_destination,
                    ) {
                        if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                            (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                        {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] Source: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total,
                                sftp_copy_source
                            )));
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] Destination: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total,
                                sftp_copy_destination
                            )));
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] Source: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                sftp_copy_source
                            )));
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] Destination: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                sftp_copy_destination
                            )));
                        }
                    }
                }
                ScenarioEvent::SftpCopyCompleted => {
                    if let (Some(step_index), Some(steps_total)) =
                        (visitor.step_index, visitor.steps_total)
                    {
                        if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                            (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                        {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] SFTP copy finished",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total
                            )));
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] SFTP copy finished",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total
                            )));
                        }
                    }
                }
                ScenarioEvent::SftpCopyProgress => {
                    if let (
                        Some(sftp_copy_progress_current),
                        Some(sftp_copy_progress_total),
                        Some(step_index),
                        Some(steps_total),
                    ) = (
                        visitor.sftp_copy_progress_current,
                        visitor.sftp_copy_progress_total,
                        visitor.step_index,
                        visitor.steps_total,
                    ) {
                        let percentage = (sftp_copy_progress_current as f64
                            / sftp_copy_progress_total as f64)
                            * 100.0;

                        if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                            (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                        {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] Progress: {:.1}%",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total,
                                percentage
                            )));
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] Progress: {:.1}%",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                percentage
                            )));
                        }
                    }
                }
                ScenarioEvent::OnFailStepsStarted => {
                    if let (Some(step_index), Some(steps_total), Some(on_fail_steps_total)) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.on_fail_steps_total,
                    ) {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} [{}/{}] [on-fail] ({}) Starting failure recovery steps",
                            SCENARIO_PREFIX,
                            step_index + 1,
                            steps_total,
                            on_fail_steps_total
                        )));
                    }
                }
                ScenarioEvent::OnFailStepsCompleted => {
                    if let (Some(step_index), Some(steps_total), Some(on_fail_steps_total)) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.on_fail_steps_total,
                    ) {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} [{}/{}] [on-fail] ({}) Failure recovery steps completed",
                            SCENARIO_PREFIX,
                            step_index + 1,
                            steps_total,
                            on_fail_steps_total
                        )));
                    }
                }
                ScenarioEvent::OnFailStepStarted => {
                    if let (
                        Some(step_index),
                        Some(steps_total),
                        Some(on_fail_step_index),
                        Some(on_fail_steps_total),
                        Some(task_description),
                    ) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.on_fail_step_index,
                        visitor.on_fail_steps_total,
                        visitor.task_description,
                    ) {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} [{}/{}] [on-fail] [{}/{}] {}",
                            SCENARIO_PREFIX,
                            step_index + 1,
                            steps_total,
                            on_fail_step_index + 1,
                            on_fail_steps_total,
                            task_description
                        )));
                    }
                }
                ScenarioEvent::StepCompleted
                | ScenarioEvent::OnFailStepCompleted
                | ScenarioEvent::CreateSessionStarted
                | ScenarioEvent::CreateSessionCompleted
                | ScenarioEvent::CreatedMockSession
                | ScenarioEvent::SessionCreated
                | ScenarioEvent::StepsStarted
                | ScenarioEvent::RemoteSudoCompleted
                | ScenarioEvent::StepsCompleted => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::trace::{
        layers::{scenario_layer::ScenarioEventLayer, EventLayer},
        AppEvent,
    };
    use std::sync::mpsc;
    use tracing::{event, span, subscriber, Level, Subscriber};
    use tracing_subscriber::{
        layer::Context, prelude::*, registry::LookupSpan, Layer, Registry,
    };

    struct TestScenarioLayer(ScenarioEventLayer);

    impl<S> Layer<S> for TestScenarioLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_new_span(
            &self,
            attrs: &tracing::span::Attributes<'_>,
            id: &tracing::span::Id,
            ctx: Context<'_, S>,
        ) {
            self.0.on_new_span(attrs, id, ctx);
        }

        fn on_record(
            &self,
            id: &tracing::span::Id,
            record: &tracing::span::Record<'_>,
            ctx: Context<'_, S>,
        ) {
            self.0.on_record(id, record, ctx);
        }

        fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
            self.0.process_event(event, ctx);
        }
    }

    fn collect_events(f: impl FnOnce()) -> Vec<AppEvent> {
        let (sender, receiver) = mpsc::channel();
        let layer = TestScenarioLayer(ScenarioEventLayer::new(sender));
        let test_subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(test_subscriber);
        f();
        receiver.try_iter().collect()
    }

    #[test]
    fn test_scenario_started_emits_log_message() {
        // Given & When
        let events = collect_events(|| {
            event!(Level::INFO, scenario.event = "scenario_started");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => assert!(msg.contains("Scenario started"), "got: {msg}"),
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_scenario_completed_emits_log_message() {
        // Given & When
        let events = collect_events(|| {
            event!(Level::INFO, scenario.event = "scenario_completed");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("Scenario completed"), "got: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_scenario_failed_emits_log_message() {
        // Given & When
        let events = collect_events(|| {
            event!(Level::INFO, scenario.event = "scenario_failed");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("Scenario failed"), "got: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_error_with_step_context_emits_formatted_message() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 3u64
            )
            .entered();
            event!(
                Level::ERROR,
                scenario.event = "error",
                scenario.error = "connection refused"
            );
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[1/3]"), "expected [1/3] in: {msg}");
                assert!(msg.contains("connection refused"), "expected error in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_error_without_context_emits_generic_message() {
        // Given & When
        let events = collect_events(|| {
            event!(Level::ERROR, scenario.event = "error");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(
                    msg.contains("Scenario execution failed"),
                    "got: {msg}"
                );
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_step_started_emits_formatted_message() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 1u64,
                steps.total = 5u64,
                task.description = "Deploy service"
            )
            .entered();
            event!(Level::INFO, scenario.event = "step_started");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[2/5]"), "expected [2/5] in: {msg}");
                assert!(msg.contains("Deploy service"), "expected description in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_remote_sudo_output_emits_header_and_plain_message() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 1u64
            )
            .entered();
            event!(
                Level::INFO,
                scenario.event = "remote_sudo_output",
                remote_sudo.output = "command output here"
            );
        });

        // Then
        assert_eq!(events.len(), 2);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("Output:"), "expected 'Output:' in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
        match &events[1] {
            AppEvent::LogPlainMessage(text) => {
                assert_eq!(text, "command output here");
            }
            other => panic!("Expected LogPlainMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_sftp_copy_started_emits_source_and_destination() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 2u64,
                sftp_copy.source = "/local/file.tar",
                sftp_copy.destination = "/remote/file.tar"
            )
            .entered();
            event!(Level::INFO, scenario.event = "sftp_copy_started");
        });

        // Then
        assert_eq!(events.len(), 2);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("/local/file.tar"), "expected source in: {msg}");
            }
            other => panic!("Expected LogMessage for source, got {other:?}"),
        }
        match &events[1] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("/remote/file.tar"), "expected dest in: {msg}");
            }
            other => panic!("Expected LogMessage for dest, got {other:?}"),
        }
    }

    #[test]
    fn test_sftp_copy_completed_emits_log_message() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 1u64
            )
            .entered();
            event!(Level::INFO, scenario.event = "sftp_copy_completed");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("SFTP copy finished"), "got: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_sftp_copy_progress_emits_percentage() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 1u64,
                sftp_copy.progress.current = 500u64,
                sftp_copy.progress.total = 1000u64
            )
            .entered();
            event!(Level::INFO, scenario.event = "sftp_copy_progress");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("50.0%"), "expected 50.0%% in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_on_fail_steps_started_emits_log_message() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 3u64,
                on_fail_steps.total = 2u64
            )
            .entered();
            event!(Level::INFO, scenario.event = "on_fail_steps_started");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(
                    msg.contains("Starting failure recovery steps"),
                    "got: {msg}"
                );
                assert!(msg.contains("(2)"), "expected on-fail count in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_step_completed_emits_nothing() {
        // Given & When
        let events = collect_events(|| {
            event!(Level::INFO, scenario.event = "step_completed");
        });

        // Then
        assert!(events.is_empty(), "expected no events for step_completed");
    }

    #[test]
    fn test_unrecognized_event_emits_nothing() {
        // Given & When
        let events = collect_events(|| {
            event!(Level::INFO, scenario.event = "totally_unknown_event_xyz");
        });

        // Then
        assert!(events.is_empty(), "expected no events for unrecognized event");
    }

    #[test]
    fn test_error_with_on_fail_step_context() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 3u64,
                on_fail_step.index = 1u64,
                on_fail_steps.total = 2u64
            )
            .entered();
            event!(
                Level::ERROR,
                scenario.event = "error",
                scenario.error = "recovery timeout"
            );
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[1/3]"), "expected [1/3] in: {msg}");
                assert!(msg.contains("[on-fail]"), "expected [on-fail] in: {msg}");
                assert!(msg.contains("[2/2]"), "expected [2/2] in: {msg}");
                assert!(msg.contains("recovery timeout"), "expected error in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_error_with_error_message_no_step_context() {
        // Given & When
        let events = collect_events(|| {
            event!(
                Level::ERROR,
                scenario.event = "error",
                scenario.error = "global failure"
            );
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("global failure"), "expected error in: {msg}");
                assert!(
                    !msg.contains("/"),
                    "should not contain step index context: {msg}"
                );
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_remote_sudo_started_emits_command() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 2u64,
                remote_sudo.command = "systemctl restart app"
            )
            .entered();
            event!(Level::INFO, scenario.event = "remote_sudo_started");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[1/2]"), "expected [1/2] in: {msg}");
                assert!(msg.contains("systemctl restart app"), "expected command in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_remote_sudo_started_with_on_fail_context() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 2u64,
                on_fail_step.index = 0u64,
                on_fail_steps.total = 1u64,
                remote_sudo.command = "recovery cmd"
            )
            .entered();
            event!(Level::INFO, scenario.event = "remote_sudo_started");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[on-fail]"), "expected [on-fail] in: {msg}");
                assert!(msg.contains("[1/1]"), "expected [1/1] in: {msg}");
                assert!(msg.contains("recovery cmd"), "expected command in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_remote_sudo_output_with_on_fail_context() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 1u64,
                on_fail_step.index = 0u64,
                on_fail_steps.total = 1u64
            )
            .entered();
            event!(
                Level::INFO,
                scenario.event = "remote_sudo_output",
                remote_sudo.output = "recovery output"
            );
        });

        // Then
        assert_eq!(events.len(), 2);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[on-fail]"), "expected [on-fail] in: {msg}");
                assert!(msg.contains("Output:"), "expected 'Output:' in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
        match &events[1] {
            AppEvent::LogPlainMessage(text) => {
                assert_eq!(text, "recovery output");
            }
            other => panic!("Expected LogPlainMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_sftp_copy_started_with_on_fail_context() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 2u64,
                on_fail_step.index = 0u64,
                on_fail_steps.total = 1u64,
                sftp_copy.source = "/backup/file.tar",
                sftp_copy.destination = "/remote/file.tar"
            )
            .entered();
            event!(Level::INFO, scenario.event = "sftp_copy_started");
        });

        // Then
        assert_eq!(events.len(), 2);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[on-fail]"), "expected [on-fail] in: {msg}");
                assert!(msg.contains("/backup/file.tar"), "expected source in: {msg}");
            }
            other => panic!("Expected LogMessage for source, got {other:?}"),
        }
        match &events[1] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[on-fail]"), "expected [on-fail] in: {msg}");
                assert!(msg.contains("/remote/file.tar"), "expected dest in: {msg}");
            }
            other => panic!("Expected LogMessage for dest, got {other:?}"),
        }
    }

    #[test]
    fn test_sftp_copy_completed_with_on_fail_context() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 1u64,
                on_fail_step.index = 0u64,
                on_fail_steps.total = 1u64
            )
            .entered();
            event!(Level::INFO, scenario.event = "sftp_copy_completed");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[on-fail]"), "expected [on-fail] in: {msg}");
                assert!(msg.contains("SFTP copy finished"), "got: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_sftp_copy_progress_with_on_fail_context() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 1u64,
                on_fail_step.index = 0u64,
                on_fail_steps.total = 1u64,
                sftp_copy.progress.current = 750u64,
                sftp_copy.progress.total = 1000u64
            )
            .entered();
            event!(Level::INFO, scenario.event = "sftp_copy_progress");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[on-fail]"), "expected [on-fail] in: {msg}");
                assert!(msg.contains("75.0%"), "expected 75.0%% in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_on_fail_steps_completed_emits_log_message() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 0u64,
                steps.total = 3u64,
                on_fail_steps.total = 2u64
            )
            .entered();
            event!(Level::INFO, scenario.event = "on_fail_steps_completed");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(
                    msg.contains("Failure recovery steps completed"),
                    "got: {msg}"
                );
                assert!(msg.contains("(2)"), "expected on-fail count in: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_on_fail_step_started_emits_log_message() {
        // Given & When
        let events = collect_events(|| {
            let _span = span!(
                Level::INFO,
                "step",
                step.index = 1u64,
                steps.total = 3u64,
                on_fail_step.index = 0u64,
                on_fail_steps.total = 2u64,
                task.description = "Rollback deployment"
            )
            .entered();
            event!(Level::INFO, scenario.event = "on_fail_step_started");
        });

        // Then
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("[2/3]"), "expected [2/3] in: {msg}");
                assert!(msg.contains("[on-fail]"), "expected [on-fail] in: {msg}");
                assert!(msg.contains("[1/2]"), "expected [1/2] in: {msg}");
                assert!(
                    msg.contains("Rollback deployment"),
                    "expected description in: {msg}"
                );
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }
}
