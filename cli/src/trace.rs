use colored::Colorize;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};
use scenario_rs::trace::ScenarioEventVisitor;
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};
use tracing::{error, info, warn, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// A tracing layer for handling and displaying scenario execution events.
///
/// This layer intercepts tracing events from the scenario execution and displays
/// them to the user in a user-friendly way, including colored text output and
/// progress bars for long-running operations.
pub struct ScenarioEventLayer {
    /// Progress bars for tracking operations, keyed by operation ID
    progress_bars: Arc<Mutex<HashMap<String, ProgressBar>>>,
}

impl ScenarioEventLayer {
    /// Creates a new ScenarioEventLayer.
    ///
    /// # Returns
    ///
    /// A new ScenarioEventLayer instance ready to be added to a tracing subscriber.
    pub fn new() -> Self {
        ScenarioEventLayer {
            progress_bars: Arc::new(Mutex::new(HashMap::new())),
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

        let mut visitor = ScenarioEventVisitor {
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
        };

        event.record(&mut visitor);

        let event_type = visitor.event_type.unwrap();

        match event_type.as_str() {
            "error" => {
                let mut bars = self.progress_bars.lock().unwrap();
                for (_, bar) in bars.drain() {
                    bar.finish_and_clear();
                }

                if let Some(error) = visitor.error {
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
                if let (Some(index), Some(total), Some(desc)) =
                    (visitor.index, visitor.total_steps, visitor.description)
                {
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
                if let Some(cmd) = visitor.command {
                    info!("{}=[{}]", "CMD".yellow(), cmd.bright_cyan());
                }
            }
            "remote_sudo_channel_output" => {
                if let Some(output) = visitor.output {
                    let trimmed = output.trim();

                    info!("{}", trimmed.chars().take(1000).collect::<String>().trim());

                    if trimmed.len() > 1000 {
                        info!("...output truncated...");
                    }
                }
            }
            "sftp_copy_started" => {
                if let (Some(source), Some(destination)) = (visitor.source, visitor.destination) {
                    info!("{}=[{}]", "SRC".yellow(), source.bright_cyan());
                    info!("{}=[{}]", "DST".yellow(), destination.bright_cyan());
                    let sftp_id = format!("sftp_{}_{}", source, destination);
                    self.get_or_create_progress_bar(&sftp_id);
                }
            }
            "sftp_copy_completed" => {
                if let (Some(source), Some(destination)) = (visitor.source, visitor.destination) {
                    let sftp_id = format!("sftp_{}_{}", source, destination);
                    self.finish_progress_bar(&sftp_id, "SFTP copy completed");
                }
            }
            "sftp_copy_progress" => {
                if let (Some(current), Some(total), Some(source), Some(destination)) = (
                    visitor.current,
                    visitor.total,
                    visitor.source,
                    visitor.destination,
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
                if let (Some(index), Some(total), Some(desc)) =
                    (visitor.index, visitor.total_steps, visitor.description)
                {
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
            _ => {}
        }
    }
}
