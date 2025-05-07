use colored::Colorize;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};
use scenario_rs::{
    trace::ScenarioEventVisitor,
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
            .any(|f| f.name() == "event")
        {
            return;
        }

        let mut visitor = ScenarioEventVisitor::default();

        event.record(&mut visitor);

        let scenario_event = visitor.scenario_event.unwrap();

        match scenario_event.as_str() {
            "error" => {
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
            "scenario_started" => {
                info!("{}", "Scenario started...".bright_blue());
            }
            "scenario_completed" => {
                info!("{}", "Scenario completed successfully!".green());
            }
            "step_started" => {
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
            "remote_sudo_started" => {
                if let Some(cmd) = visitor.remote_sudo_command {
                    info!("{}=[{}]", "CMD".yellow(), cmd.bright_cyan());
                }
            }
            "remote_sudo_output" => {
                if let Some(output) = visitor.remote_sudo_output {
                    let trimmed = output.trim();

                    info!("{}", trimmed.chars().take(1000).collect::<String>().trim());

                    if trimmed.len() > 1000 {
                        info!("...output truncated...");
                    }
                }
            }
            "sftp_copy_started" => {
                if let (Some(source), Some(destination)) =
                    (visitor.sftp_copy_source, visitor.sftp_copy_destination)
                {
                    info!("{}=[{}]", "SRC".yellow(), source.bright_cyan());
                    info!("{}=[{}]", "DST".yellow(), destination.bright_cyan());
                    let sftp_id = format!("sftp_{}_{}", source, destination);
                    self.get_or_create_progress_bar(&sftp_id);
                }
            }
            "sftp_copy_completed" => {
                if let (Some(source), Some(destination)) =
                    (visitor.sftp_copy_source, visitor.sftp_copy_destination)
                {
                    let sftp_id = format!("sftp_{}_{}", source, destination);
                    self.finish_progress_bar(&sftp_id, "SFTP copy completed");
                }
            }
            "sftp_copy_progress" => {
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
            "on_fail_steps_started" => {
                warn!("{}", "On-fail steps started...".red());
            }
            "on_fail_steps_completed" => {
                info!("{}", "On-fail steps completed".green());
            }
            "on_fail_step_started" => {
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
            "create_session_started" => {}
            "created_mock_session" => {}
            "session_created" => {}
            "steps_started" => {}
            "step_completed" => {}
            "remote_sudo_completed" => {}
            "steps_completed" => {}
            "on_fail_step_completed" => {}
            _ => {
                error!("Unrecognized event type: {}", scenario_event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::trace::ScenarioEventLayer;
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
        error!(scenario.event = "error", scenario.error = "Test error");

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
            scenario.event = "sftp_copy_started",
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
            scenario.event = "sftp_copy_progress",
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
            scenario.event = "sftp_copy_completed",
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
}
