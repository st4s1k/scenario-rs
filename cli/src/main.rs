use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};
use scenario_rs::scenario::events::Event;
use scenario_rs::scenario::Scenario;
use std::sync::mpsc::channel;
use std::{path::PathBuf, process};
use tracing::{debug, error, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "JSON_FILE")]
    config_path: PathBuf,
}

const SEPARATOR: &'static str = "------------------------------------------------------------";

fn main() {
    let _tracing_guard = FmtSubscriber::builder().compact().without_time().init();

    let cli: Cli = Cli::parse();

    let scenario: Scenario = match Scenario::try_from(cli.config_path) {
        Ok(scenario) => scenario,
        Err(error) => {
            error!("{}", SEPARATOR);
            error!("Scenario initialization failed: {}", error);
            error!("{}", SEPARATOR);
            process::exit(1);
        }
    };

    // Create a channel for events.
    let (tx, rx) = channel();

    // Spawn scenario execution in a separate thread.
    std::thread::spawn(move || match scenario.execute(tx.clone()) {
        Ok(_) => {
            info!("{}", SEPARATOR);
            info!("{}", "Scenario completed successfully!".cyan());
            info!("{}", SEPARATOR);
        }
        Err(error) => {
            error!("{}", SEPARATOR);
            error!("Scenario execution failed: {}", error);
            error!("{}", SEPARATOR);
            tx.send(Event::ScenarioError(error.to_string())).unwrap();
        }
    });

    // Process events as they come in.
    let mut active_progress_bar: Option<ProgressBar> = None;

    for event in rx {
        match event {
            Event::ScenarioStarted => {
                info!("Scenario started...");
            }
            Event::StepStarted {
                index,
                total_steps,
                description,
            } => {
                info!("{}", SEPARATOR);
                info!(
                    "{}",
                    format!("[{}/{}] {}", index + 1, total_steps, description).purple()
                );
            }
            Event::RemoteSudoBefore(cmd) => {
                info!("{}", "Executing:".yellow());
                info!("{}", cmd.bold());
            }
            Event::RemoteSudoChannelOutput(output) => {
                let trimmed = output.trim();
                info!("{}", trimmed.chars().take(1000).collect::<String>().trim());
                if trimmed.len() > 1000 {
                    debug!("{}", trimmed);
                    info!("...output truncated...");
                }
            }
            Event::RemoteSudoAfter => {
                info!("Remote sudo command completed");
            }
            Event::SftpCopyBefore {
                source,
                destination,
            } => {
                info!("{}", "Source:".yellow());
                info!("{}", source.bold());
                info!("{}", "Destination:".yellow());
                info!("{}", destination.bold());

                // Initialize a new progress bar
                let pb = ProgressBar::new(100);
                pb.set_draw_target(ProgressDrawTarget::stderr());
                pb.set_style(ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})"
                ).unwrap()
                .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| 
                    write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
                .progress_chars("#>-"));

                active_progress_bar = Some(pb);
            }
            Event::SftpCopyProgress { current, total } => {
                if let Some(pb) = &active_progress_bar {
                    match pb.length() {
                        Some(len) => {
                            if len != total {
                                pb.set_length(total);
                            }
                        }
                        None => {
                            pb.set_length(total);
                        }
                    }
                    pb.set_position(current);
                }
            }
            Event::SftpCopyAfter => {
                if let Some(pb) = active_progress_bar.take() {
                    pb.finish_with_message("SFTP copy completed");
                }
                info!("SFTP copy finished");
            }
            Event::OnFailStepsStarted => {
                info!("{}", SEPARATOR);
                info!("On-fail steps started");
            }
            Event::OnFailStepStarted {
                index,
                total_steps,
                description,
            } => {
                info!("{}", SEPARATOR);
                info!(
                    "{}",
                    format!("[on-fail] [{}/{}] {}", index + 1, total_steps, description).purple()
                );
            }
            Event::OnFailStepCompleted => {
                info!("On-fail step completed");
            }
            Event::OnFailStepsCompleted => {
                info!("{}", SEPARATOR);
                info!("On-fail steps completed");
            }
            Event::StepCompleted => {
                info!("Step completed");
            }
            Event::ScenarioCompleted => {
                info!("{}", SEPARATOR);
                info!("{}", "Scenario completed successfully!".cyan());
                info!("{}", SEPARATOR);
            }
            Event::ScenarioError(msg) => {
                // Make sure to clean up any active progress bar before showing error
                if let Some(pb) = active_progress_bar.take() {
                    pb.finish_and_clear();
                }

                error!("{}", SEPARATOR);
                error!("Scenario execution error: {}", msg);
                error!("{}", SEPARATOR);
                process::exit(1);
            }
        }
    }
}
