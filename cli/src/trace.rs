use colored::Colorize;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};
use scenario_rs::{
    trace::{ScenarioEvent, ScenarioEventVisitor, SCENARIO_EVENT_FIELD},
    utils::{ArcMutex, Wrap},
};
use std::{collections::HashMap, fmt};
use tracing::{error, info, warn, Subscriber};
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
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})").unwrap()
            .progress_chars("#>-")
            .with_key("eta", |state: &ProgressState, w: &mut dyn fmt::Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            })
        );

        bars.insert(id.to_string(), pb.clone());
        pb
    }

    /// Completes a progress bar with a final message and removes it.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier for the progress bar to complete
    /// * `message` - The message to display when the progress bar completes
    fn finish_progress_bar(&self, id: &str, message: &str) {
        let mut bars = self.progress_bars.lock().unwrap();
        if let Some(bar) = bars.remove(id) {
            bar.finish_with_message(message.to_owned());
        }
    }
}

impl<S> Layer<S> for ScenarioEventLayer
where
    S: Subscriber + for<'a> LookupSpan<'a> + Send + Sync + 'static,
{
    /// Processes tracing events and formats them for user display.
    ///
    /// This method intercepts events with an "event" field and formats them
    /// appropriately based on their type, including creating progress bars
    /// for file transfers, displaying command outputs, and formatting errors.
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
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

        let scenario_event_str = visitor.scenario_event.unwrap();
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

                if let Some(error) = visitor.scenario_error {
                    error!("{}", error);
                } else {
                    error!("{}", "Scenario execution error".red());
                }
            }
            ScenarioEvent::ScenarioStarted => {
                info!("{}", "Scenario started...".bright_blue());
            }
            ScenarioEvent::ScenarioCompleted => {
                info!("{}", "Scenario completed successfully!".green());
            }
            ScenarioEvent::StepStarted => {
                if let (Some(index), Some(total), Some(desc)) = (
                    visitor.step_index,
                    visitor.steps_total,
                    visitor.task_description,
                ) {
                    info!(
                        "{}=[{}] {}=[{}] {}=[{}]",
                        "STEP".yellow(),
                        format!("{}", index + 1).purple(),
                        "TOTAL".yellow(),
                        format!("{}", total).purple(),
                        "DESC".yellow(),
                        desc.purple()
                    );
                }
            }
            ScenarioEvent::RemoteSudoStarted => {
                if let Some(cmd) = visitor.remote_sudo_command {
                    info!("{}=[{}]", "CMD".yellow(), cmd.bright_cyan());
                }
            }
            ScenarioEvent::RemoteSudoOutput => {
                if let Some(output) = visitor.remote_sudo_output {
                    let trimmed = output.trim();

                    info!("{}", trimmed.chars().take(1000).collect::<String>().trim());

                    if trimmed.len() > 1000 {
                        info!("...output truncated...");
                    }
                }
            }
            ScenarioEvent::SftpCopyStarted => {
                if let (Some(source), Some(destination)) =
                    (visitor.sftp_copy_source, visitor.sftp_copy_destination)
                {
                    info!("{}=[{}]", "SRC".yellow(), source.bright_cyan());
                    info!("{}=[{}]", "DST".yellow(), destination.bright_cyan());
                    let sftp_id = format!("sftp_{}_{}", source, destination);
                    self.get_or_create_progress_bar(&sftp_id);
                }
            }
            ScenarioEvent::SftpCopyCompleted => {
                if let (Some(source), Some(destination)) =
                    (visitor.sftp_copy_source, visitor.sftp_copy_destination)
                {
                    let sftp_id = format!("sftp_{}_{}", source, destination);
                    self.finish_progress_bar(&sftp_id, "SFTP copy completed");
                }
            }
            ScenarioEvent::SftpCopyProgress => {
                if let (Some(current), Some(total), Some(source), Some(destination)) = (
                    visitor.sftp_copy_progress_current,
                    visitor.sftp_copy_progress_total,
                    visitor.sftp_copy_source,
                    visitor.sftp_copy_destination,
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
                warn!("{}", "On-fail steps started...".red());
            }
            ScenarioEvent::OnFailStepsCompleted => {
                info!("{}", "On-fail steps completed".green());
            }
            ScenarioEvent::OnFailStepStarted => {
                if let (Some(index), Some(total), Some(desc)) = (
                    visitor.on_fail_step_index,
                    visitor.on_fail_steps_total,
                    visitor.task_description,
                ) {
                    info!(
                        "{}=[{}] {}=[{}] {}=[{}]",
                        "STEP".yellow(),
                        format!("{}", index + 1).purple(),
                        "TOTAL".yellow(),
                        format!("{}", total).purple(),
                        "DESC".yellow(),
                        desc.purple()
                    );
                }
            }
            ScenarioEvent::CreateSessionStarted
            | ScenarioEvent::CreateSessionCompleted
            | ScenarioEvent::CreatedMockSession
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

#[cfg(test)]
mod tests {
    use crate::trace::ScenarioEventLayer;
    use scenario_rs::trace::ScenarioEvent;
    use tracing::{error, info, subscriber};
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
        layer.finish_progress_bar(id, "Test complete");

        // Then
        assert_eq!(layer.progress_bars.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_scenarioeventlayer_finish_progress_bar_nonexistent() {
        // Given
        let layer = ScenarioEventLayer::new();

        // When
        layer.finish_progress_bar("nonexistent", "Test complete");

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
        error!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = "Test error");

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
            sftp_copy.destination = "/remote/file.txt"
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

        // When & Then (no panic)
        info!(scenario.event = ScenarioEvent::ScenarioStarted.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_scenario_completed() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then (no panic)
        info!(scenario.event = ScenarioEvent::ScenarioCompleted.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_step_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then (no panic)
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

        // When & Then (no panic)
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

        // When & Then (no panic)
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

        // When & Then (no panic, output truncated to 1000 chars)
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

        // When & Then (no panic)
        info!(scenario.event = ScenarioEvent::OnFailStepsStarted.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_on_fail_steps_completed() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then (no panic)
        info!(scenario.event = ScenarioEvent::OnFailStepsCompleted.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_on_fail_step_started() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then (no panic)
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

        // When & Then (no panic, uses generic error message)
        error!(scenario.event = ScenarioEvent::Error.as_str());
    }

    #[test]
    fn test_scenarioeventlayer_on_event_noop_events() {
        // Given
        let layer = ScenarioEventLayer::new();
        let subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(subscriber);

        // When & Then (no panic, these are intentionally no-ops)
        info!(scenario.event = ScenarioEvent::CreateSessionStarted.as_str());
        info!(scenario.event = ScenarioEvent::CreateSessionCompleted.as_str());
        info!(scenario.event = ScenarioEvent::CreatedMockSession.as_str());
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

        // When & Then (no panic, just logs error)
        error!(scenario.event = "totally_unknown_event_xyz");
    }
}
