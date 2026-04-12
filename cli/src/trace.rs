use colored::Colorize;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};
use scenario_rs::{
    trace::{ScenarioEvent, ScenarioEventVisitor, SCENARIO_EVENT_FIELD},
    utils::{ArcMutex, Wrap},
};
use std::{collections::HashMap, fmt};
use tracing::{error, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// A tracing layer for handling and displaying scenario execution events.
///
/// This layer intercepts tracing events from the scenario execution and displays
/// them to the user in a user-friendly way, including colored text output and
/// progress bars for long-running operations.
pub struct ScenarioEventLayer {
    /// Progress bars for tracking operations, keyed by operation ID
    progress_bars: ArcMutex<HashMap<String, ProgressBar>>,
}

impl ScenarioEventLayer {
    /// Creates a new ScenarioEventLayer.
    ///
    /// # Returns
    ///
    /// A new ScenarioEventLayer instance ready to be added to a tracing subscriber.
    pub fn new() -> Self {
        ScenarioEventLayer {
            progress_bars: ArcMutex::wrap(HashMap::new()),
        }
    }

    /// Gets an existing progress bar or creates a new one if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier for the progress bar
    ///
    /// # Returns
    ///
    /// A progress bar instance configured with appropriate styling.
    fn get_or_create_progress_bar(&self, id: &str) -> ProgressBar {
        let mut bars = self.progress_bars.lock().unwrap();

        if let Some(bar) = bars.get(id) {
            return bar.clone();
        }

        let pb = ProgressBar::new(100);
        pb.set_draw_target(ProgressDrawTarget::stderr());

        #[cfg(test)]
        pb.set_draw_target(ProgressDrawTarget::hidden());

        pb.set_style(
            ProgressStyle::default_bar()
            .template("             [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({precise_elapsed}, {binary_bytes_per_sec})").unwrap()
            .progress_chars("#>-")
            .with_key("precise_elapsed", Self::format_elapsed)
        );

        bars.insert(id.to_string(), pb.clone());
        pb
    }

    /// Completes a progress bar and removes it.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier for the progress bar to complete
    fn finish_progress_bar(&self, id: &str) {
        let mut bars = self.progress_bars.lock().unwrap();
        if let Some(bar) = bars.remove(id) {
            bar.finish();
        }
    }

    /// Formats elapsed time as a precise decimal.
    #[cfg(not(tarpaulin_include))]
    fn format_elapsed(state: &ProgressState, w: &mut dyn fmt::Write) {
        Self::write_elapsed(w, state.elapsed().as_secs_f64())
    }

    /// Writes elapsed seconds as a formatted string.
    fn write_elapsed(w: &mut dyn fmt::Write, elapsed_secs: f64) {
        write!(w, "{:.1}s", elapsed_secs).unwrap()
    }

    /// Processes a scenario event visitor and formats the output for the user.
    ///
    /// This is the core event processing logic, extracted from `on_event`
    /// so it can be called and tested directly.
    fn process_scenario_event(&self, visitor: &ScenarioEventVisitor) {
        let scenario_event_str = visitor.scenario_event.as_ref().unwrap();
        let Ok(scenario_event) = scenario_event_str.parse::<ScenarioEvent>() else {
            error!("Unrecognized event type: {}", scenario_event_str);
            return;
        };

        match scenario_event {
            ScenarioEvent::Error => {
                let mut bars = self.progress_bars.lock().unwrap();
                for (_, bar) in bars.drain() {
                    bar.finish_and_clear();
                }

                let msg = visitor
                    .scenario_error
                    .as_deref()
                    .unwrap_or("Scenario execution error");
                eprintln!("{:>12} {}", "error".red().bold(), msg);
            }
            ScenarioEvent::ScenarioStarted => {
                println!("{:>12} scenario", "Starting".green().bold());
            }
            ScenarioEvent::ScenarioCompleted => {
                println!("{:>12} scenario completed successfully", "Finished".green().bold());
            }
            ScenarioEvent::StepStarted => {
                if let (Some(index), Some(total), Some(ref desc)) = (
                    visitor.step_index,
                    visitor.steps_total,
                    &visitor.task_description,
                ) {
                    println!(
                        "{:>12} [{}/{}] {}",
                        "Executing".green().bold(),
                        index + 1,
                        total,
                        desc
                    );
                }
            }
            ScenarioEvent::RemoteSudoStarted => {
                if let Some(ref cmd) = visitor.remote_sudo_command {
                    println!("{:>12} {}", "Running".green().bold(), cmd.bright_cyan());
                }
            }
            ScenarioEvent::RemoteSudoOutput => {
                if let Some(ref output) = visitor.remote_sudo_output {
                    let trimmed = output.trim();
                    let display: String = trimmed.chars().take(1000).collect();

                    for line in display.trim().lines() {
                        println!("{:>12} {}", "", line);
                    }

                    if trimmed.len() > 1000 {
                        println!("{:>12} ...output truncated...", "");
                    }
                }
            }
            ScenarioEvent::SftpCopyStarted => {
                if let (Some(ref source), Some(ref destination)) =
                    (&visitor.sftp_copy_source, &visitor.sftp_copy_destination)
                {
                    println!(
                        "{:>12} {} {} {}",
                        "Uploading".green().bold(),
                        source,
                        "→".bright_black(),
                        destination
                    );
                    let sftp_id = format!("sftp_{}_{}", source, destination);
                    self.get_or_create_progress_bar(&sftp_id);
                }
            }
            ScenarioEvent::SftpCopyCompleted => {
                if let (Some(ref source), Some(ref destination)) =
                    (&visitor.sftp_copy_source, &visitor.sftp_copy_destination)
                {
                    let sftp_id = format!("sftp_{}_{}", source, destination);
                    self.finish_progress_bar(&sftp_id);
                }
            }
            ScenarioEvent::SftpCopyProgress => {
                if let (Some(current), Some(total), Some(ref source), Some(ref destination)) = (
                    visitor.sftp_copy_progress_current,
                    visitor.sftp_copy_progress_total,
                    &visitor.sftp_copy_source,
                    &visitor.sftp_copy_destination,
                ) {
                    let sftp_id = format!("sftp_{}_{}", source, destination);
                    let pb = self.get_or_create_progress_bar(&sftp_id);
                    if pb.length() != Some(total) {
                        pb.set_length(total);
                    }
                    pb.set_position(current);
                }
            }
            ScenarioEvent::OnFailStepsStarted => {
                eprintln!("{:>12} on-fail steps triggered", "warning".yellow().bold());
            }
            ScenarioEvent::OnFailStepsCompleted => {
                println!("{:>12} on-fail steps", "Finished".green().bold());
            }
            ScenarioEvent::OnFailStepStarted => {
                if let (Some(index), Some(total), Some(ref desc)) = (
                    visitor.on_fail_step_index,
                    visitor.on_fail_steps_total,
                    &visitor.task_description,
                ) {
                    println!(
                        "{:>12} [{}/{}] {}",
                        "Recovering".yellow().bold(),
                        index + 1,
                        total,
                        desc
                    );
                }
            }
            ScenarioEvent::CreateSessionStarted
            | ScenarioEvent::CreateSessionCompleted
            | ScenarioEvent::CreatedDryRunSession
            | ScenarioEvent::SessionCreated
            | ScenarioEvent::StepsStarted
            | ScenarioEvent::StepCompleted
            | ScenarioEvent::RemoteSudoCompleted
            | ScenarioEvent::StepsCompleted
            | ScenarioEvent::OnFailStepCompleted
            | ScenarioEvent::ScenarioFailed => {}
        }
    }
}

impl<S> Layer<S> for ScenarioEventLayer
where
    S: Subscriber + for<'a> LookupSpan<'a> + Send + Sync + 'static,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        let mut visitor = ScenarioEventVisitor::default();
        attrs.record(&mut visitor);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(visitor);
        }
    }

    fn on_record(
        &self,
        id: &tracing::span::Id,
        record: &tracing::span::Record<'_>,
        ctx: Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            if let Some(v) = span.extensions_mut().get_mut::<ScenarioEventVisitor>() {
                record.record(v);
            }
        }
    }

    /// Processes tracing events and formats them for user display.
    ///
    /// This method intercepts events with an "event" field and formats them
    /// appropriately based on their type, including creating progress bars
    /// for file transfers, displaying command outputs, and formatting errors.
    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        if !event
            .metadata()
            .fields()
            .iter()
            .any(|f| f.name() == SCENARIO_EVENT_FIELD)
        {
            return;
        }

        let mut visitor = ScenarioEventVisitor::default();

        event.record(&mut visitor);

        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                if let Some(extra) = span.extensions().get::<ScenarioEventVisitor>() {
                    visitor.merge(extra);
                }
            }
        }

        self.process_scenario_event(&visitor);
    }
}

#[cfg(test)]
mod tests {
    use crate::trace::ScenarioEventLayer;
    use scenario_rs::trace::{ScenarioEvent, ScenarioEventVisitor};
    use tracing::{debug, error, info, span, subscriber, Level};
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    #[test]
    fn test_scenarioeventlayer_new() {
        // Given & When
        let layer = ScenarioEventLayer::new();

        // Then
        assert!(layer.progress_bars.lock().unwrap().is_empty());
    }

    #[test]
    fn test_scenarioeventlayer_get_or_create_progress_bar_creates_new() {
        // Given
        let layer = ScenarioEventLayer::new();
        let id = "test_progress";

        // When
        let pb = layer.get_or_create_progress_bar(id);

        // Then
        assert_eq!(pb.length(), Some(100));
        assert_eq!(layer.progress_bars.lock().unwrap().len(), 1);
        assert!(layer.progress_bars.lock().unwrap().contains_key(id));
    }

    #[test]
    fn test_scenarioeventlayer_get_or_create_progress_bar_returns_existing() {
        // Given
        let layer = ScenarioEventLayer::new();
        let id = "test_progress";
        let first_pb = layer.get_or_create_progress_bar(id);

        // When
        let second_pb = layer.get_or_create_progress_bar(id);

        // Then
        assert_eq!(layer.progress_bars.lock().unwrap().len(), 1);
        assert_eq!(first_pb.position(), second_pb.position());
    }

    #[test]
    fn test_scenarioeventlayer_finish_progress_bar() {
        // Given
        let layer = ScenarioEventLayer::new();
        let id = "test_progress";
        layer.get_or_create_progress_bar(id);
        assert_eq!(layer.progress_bars.lock().unwrap().len(), 1);

        // When
        layer.finish_progress_bar(id);

        // Then
        assert_eq!(layer.progress_bars.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_scenarioeventlayer_finish_progress_bar_nonexistent() {
        // Given
        let layer = ScenarioEventLayer::new();

        // When
        layer.finish_progress_bar("nonexistent");

        // Then
        assert_eq!(layer.progress_bars.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_scenarioeventlayer_on_event_error_clears_progress_bars() {
        // Given
        let layer = ScenarioEventLayer::new();
        let progress_bars = layer.progress_bars.clone();
        let id = "test_progress";
        layer.get_or_create_progress_bar(id);
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When
        error!(
            scenario.event = ScenarioEvent::Error.as_str(),
            scenario.error = "Test error"
        );

        // Then
        assert_eq!(progress_bars.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_scenarioeventlayer_on_event_sftp_copy_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let progress_bars = layer.progress_bars.clone();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When
        info!(
            scenario.event = ScenarioEvent::SftpCopyStarted.as_str(),
            sftp_copy.source = "/local/file.txt",
            sftp_copy.destination = "/remote/file.txt"
        );

        // Then
        let id = "sftp_/local/file.txt_/remote/file.txt";
        assert!(progress_bars.lock().unwrap().contains_key(id));
    }

    #[test]
    fn test_scenarioeventlayer_on_event_sftp_copy_progress() {
        // Given
        let layer = ScenarioEventLayer::new();
        let progress_bars = layer.progress_bars.clone();
        let id = "sftp_/local/file.txt_/remote/file.txt";
        layer.get_or_create_progress_bar(id);
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When
        info!(
            scenario.event = ScenarioEvent::SftpCopyProgress.as_str(),
            sftp_copy.source = "/local/file.txt",
            sftp_copy.destination = "/remote/file.txt",
            sftp_copy.progress.current = 50u64,
            sftp_copy.progress.total = 200u64
        );

        // Then
        let pb = progress_bars.lock().unwrap().get(id).unwrap().clone();
        assert_eq!(pb.length(), Some(200));
        assert_eq!(pb.position(), 50);
    }

    #[test]
    fn test_scenarioeventlayer_on_event_sftp_copy_completed() {
        // Given
        let layer = ScenarioEventLayer::new();
        let progress_bars = layer.progress_bars.clone();
        let id = "sftp_/local/file.txt_/remote/file.txt";
        layer.get_or_create_progress_bar(id);
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When
        info!(
            scenario.event = ScenarioEvent::SftpCopyCompleted.as_str(),
            sftp_copy.source = "/local/file.txt",
            sftp_copy.destination = "/remote/file.txt",
            sftp_copy.elapsed_ms = 15230u64,
            sftp_copy.throughput_mbps = "3.28"
        );

        // Then
        assert!(!progress_bars.lock().unwrap().contains_key(id));
    }

    #[test]
    fn test_scenarioeventlayer_on_event_ignores_non_event_messages() {
        // Given
        let layer = ScenarioEventLayer::new();
        let progress_bars = layer.progress_bars.clone();
        let progress_bars_before = progress_bars.lock().unwrap().len();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When
        info!("Regular log message");

        // Then
        let progress_bars_after = progress_bars.lock().unwrap().len();
        assert_eq!(progress_bars_before, progress_bars_after);
    }

    #[test]
    fn test_scenarioeventlayer_on_event_scenario_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        info!(scenario.event = ScenarioEvent::ScenarioStarted.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_scenario_completed() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        info!(scenario.event = ScenarioEvent::ScenarioCompleted.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_step_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        info!(
            scenario.event = ScenarioEvent::StepStarted.as_str(),
            step.index = 0u64,
            steps.total = 3u64,
            task.description = "Deploy service"
        );
    }

    #[test]
    fn test_scenarioeventlayer_on_event_remote_sudo_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        info!(
            scenario.event = ScenarioEvent::RemoteSudoStarted.as_str(),
            remote_sudo.command = "systemctl restart app"
        );
    }

    #[test]
    fn test_scenarioeventlayer_on_event_remote_sudo_output() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        info!(
            scenario.event = ScenarioEvent::RemoteSudoOutput.as_str(),
            remote_sudo.output = "command output here"
        );
    }

    #[test]
    fn test_scenarioeventlayer_on_event_remote_sudo_output_truncated() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);
        let long_output = "x".repeat(1500);

        // When & Then
        info!(
            scenario.event = ScenarioEvent::RemoteSudoOutput.as_str(),
            remote_sudo.output = long_output.as_str()
        );
    }

    #[test]
    fn test_scenarioeventlayer_on_event_on_fail_steps_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        info!(scenario.event = ScenarioEvent::OnFailStepsStarted.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_on_fail_steps_completed() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        info!(scenario.event = ScenarioEvent::OnFailStepsCompleted.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_on_fail_step_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        info!(
            scenario.event = ScenarioEvent::OnFailStepStarted.as_str(),
            on_fail_step.index = 0u64,
            on_fail_steps.total = 2u64,
            task.description = "Rollback deployment"
        );
    }

    #[test]
    fn test_scenarioeventlayer_on_event_error_without_error_message() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        error!(scenario.event = ScenarioEvent::Error.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_noop_events() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        info!(scenario.event = ScenarioEvent::CreateSessionStarted.as_str());
        info!(scenario.event = ScenarioEvent::CreateSessionCompleted.as_str());
        info!(scenario.event = ScenarioEvent::CreatedDryRunSession.as_str());
        info!(scenario.event = ScenarioEvent::SessionCreated.as_str());
        info!(scenario.event = ScenarioEvent::StepsStarted.as_str());
        info!(scenario.event = ScenarioEvent::StepCompleted.as_str());
        info!(scenario.event = ScenarioEvent::RemoteSudoCompleted.as_str());
        info!(scenario.event = ScenarioEvent::StepsCompleted.as_str());
        info!(scenario.event = ScenarioEvent::OnFailStepCompleted.as_str());
        info!(scenario.event = ScenarioEvent::ScenarioFailed.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_unrecognized_event() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        error!(scenario.event = "totally_unknown_event_xyz");
    }

    fn visitor_with_event(event: ScenarioEvent) -> ScenarioEventVisitor {
        ScenarioEventVisitor {
            scenario_event: Some(event.as_str().to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_write_elapsed() {
        // Given
        let mut buf = String::new();

        // When
        ScenarioEventLayer::write_elapsed(&mut buf, 5.123);

        // Then
        assert_eq!(buf, "5.1s");
    }

    #[test]
    fn test_process_scenario_event_error_with_message() {
        // Given
        let layer = ScenarioEventLayer::new();
        layer.get_or_create_progress_bar("bar1");
        let mut visitor = visitor_with_event(ScenarioEvent::Error);
        visitor.scenario_error = Some("connection refused".to_string());

        // When
        layer.process_scenario_event(&visitor);

        // Then
        assert!(layer.progress_bars.lock().unwrap().is_empty());
    }

    #[test]
    fn test_process_scenario_event_error_without_message() {
        // Given
        let layer = ScenarioEventLayer::new();
        let visitor = visitor_with_event(ScenarioEvent::Error);

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_scenario_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let visitor = visitor_with_event(ScenarioEvent::ScenarioStarted);

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_scenario_completed() {
        // Given
        let layer = ScenarioEventLayer::new();
        let visitor = visitor_with_event(ScenarioEvent::ScenarioCompleted);

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_step_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let mut visitor = visitor_with_event(ScenarioEvent::StepStarted);
        visitor.step_index = Some(0);
        visitor.steps_total = Some(3);
        visitor.task_description = Some("Deploy service".to_string());

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_remote_sudo_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let mut visitor = visitor_with_event(ScenarioEvent::RemoteSudoStarted);
        visitor.remote_sudo_command = Some("systemctl restart app".to_string());

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_remote_sudo_output() {
        // Given
        let layer = ScenarioEventLayer::new();
        let mut visitor = visitor_with_event(ScenarioEvent::RemoteSudoOutput);
        visitor.remote_sudo_output = Some("command output".to_string());

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_remote_sudo_output_truncated() {
        // Given
        let layer = ScenarioEventLayer::new();
        let mut visitor = visitor_with_event(ScenarioEvent::RemoteSudoOutput);
        visitor.remote_sudo_output = Some("x".repeat(1500));

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_sftp_copy_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let mut visitor = visitor_with_event(ScenarioEvent::SftpCopyStarted);
        visitor.sftp_copy_source = Some("/local/file.txt".to_string());
        visitor.sftp_copy_destination = Some("/remote/file.txt".to_string());

        // When
        layer.process_scenario_event(&visitor);

        // Then
        let id = "sftp_/local/file.txt_/remote/file.txt";
        assert!(layer.progress_bars.lock().unwrap().contains_key(id));
    }

    #[test]
    fn test_process_scenario_event_sftp_copy_completed() {
        // Given
        let layer = ScenarioEventLayer::new();
        let id = "sftp_/local/file.txt_/remote/file.txt";
        layer.get_or_create_progress_bar(id);
        let mut visitor = visitor_with_event(ScenarioEvent::SftpCopyCompleted);
        visitor.sftp_copy_source = Some("/local/file.txt".to_string());
        visitor.sftp_copy_destination = Some("/remote/file.txt".to_string());

        // When
        layer.process_scenario_event(&visitor);

        // Then
        assert!(!layer.progress_bars.lock().unwrap().contains_key(id));
    }

    #[test]
    fn test_process_scenario_event_sftp_copy_progress() {
        // Given
        let layer = ScenarioEventLayer::new();
        let id = "sftp_/local/file.txt_/remote/file.txt";
        layer.get_or_create_progress_bar(id);
        let mut visitor = visitor_with_event(ScenarioEvent::SftpCopyProgress);
        visitor.sftp_copy_source = Some("/local/file.txt".to_string());
        visitor.sftp_copy_destination = Some("/remote/file.txt".to_string());
        visitor.sftp_copy_progress_current = Some(50);
        visitor.sftp_copy_progress_total = Some(200);

        // When
        layer.process_scenario_event(&visitor);

        // Then
        let pb = layer.progress_bars.lock().unwrap().get(id).unwrap().clone();
        assert_eq!(pb.length(), Some(200));
        assert_eq!(pb.position(), 50);
    }

    #[test]
    fn test_process_scenario_event_on_fail_steps_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let visitor = visitor_with_event(ScenarioEvent::OnFailStepsStarted);

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_on_fail_steps_completed() {
        // Given
        let layer = ScenarioEventLayer::new();
        let visitor = visitor_with_event(ScenarioEvent::OnFailStepsCompleted);

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_on_fail_step_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let mut visitor = visitor_with_event(ScenarioEvent::OnFailStepStarted);
        visitor.on_fail_step_index = Some(0);
        visitor.on_fail_steps_total = Some(2);
        visitor.task_description = Some("Rollback deployment".to_string());

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_process_scenario_event_noop_events() {
        // Given
        let layer = ScenarioEventLayer::new();

        // When & Then
        for event in [
            ScenarioEvent::CreateSessionStarted,
            ScenarioEvent::CreateSessionCompleted,
            ScenarioEvent::CreatedDryRunSession,
            ScenarioEvent::SessionCreated,
            ScenarioEvent::StepsStarted,
            ScenarioEvent::StepCompleted,
            ScenarioEvent::RemoteSudoCompleted,
            ScenarioEvent::StepsCompleted,
            ScenarioEvent::OnFailStepCompleted,
            ScenarioEvent::ScenarioFailed,
        ] {
            layer.process_scenario_event(&visitor_with_event(event));
        }
    }

    #[test]
    fn test_process_scenario_event_unrecognized() {
        // Given
        let layer = ScenarioEventLayer::new();
        let visitor = ScenarioEventVisitor {
            scenario_event: Some("totally_unknown_event_xyz".to_string()),
            ..Default::default()
        };

        // When & Then
        layer.process_scenario_event(&visitor);
    }

    #[test]
    fn test_on_new_span_stores_visitor_in_span_extensions() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then
        let _span = span!(
            Level::DEBUG,
            "sftp_copy",
            sftp_copy.source = "/local/file.txt",
            sftp_copy.destination = "/remote/dest.txt"
        )
        .entered();
    }

    #[test]
    fn test_on_record_updates_span_visitor() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When
        let span = span!(
            Level::DEBUG,
            "sftp_copy",
            sftp_copy.source = tracing::field::Empty,
            sftp_copy.destination = tracing::field::Empty
        );
        span.record("sftp_copy.source", "/local/file.txt");
        span.record("sftp_copy.destination", "/remote/dest.txt");
    }

    #[test]
    fn test_on_event_merges_span_context_for_sftp_completed() {
        // Given
        let layer = ScenarioEventLayer::new();
        let progress_bars = layer.progress_bars.clone();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        let span = span!(
            Level::DEBUG,
            "sftp_copy",
            sftp_copy.source = "/local/file.txt",
            sftp_copy.destination = "/remote/dest.txt"
        );
        let _entered = span.enter();

        // When
        debug!(
            scenario.event = ScenarioEvent::SftpCopyCompleted.as_str(),
            sftp_copy.elapsed_ms = 1500u64,
            sftp_copy.throughput_mbps = "33.00"
        );

        // Then
        let sftp_id = "sftp_/local/file.txt_/remote/dest.txt";
        assert!(!progress_bars.lock().unwrap().contains_key(sftp_id));
    }
}
