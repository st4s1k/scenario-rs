use clap::Parser;
use colored::Colorize;
use indicatif::{
    ProgressBar,
    ProgressDrawTarget,
    ProgressState,
    ProgressStyle,
};
use scenario_rs::scenario::rollback::RollbackSteps;
use scenario_rs::{
    config::ScenarioConfig,
    scenario::{
        lifecycle::{
            ExecutionLifecycle,
            RemoteSudoLifecycle,
            RollbackLifecycle,
            RollbackStepLifecycle,
            SftpCopyLifecycle,
            StepsLifecycle,
        },
        remote_sudo::RemoteSudo,
        sftp_copy::SftpCopy,
        task::Task,
        Scenario,
    },
};
use std::{fs::File, io::Read, path::PathBuf, process};
use tracing::{debug, error, info, warn};
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

    let config = ScenarioConfig::try_from(cli.config_path)
        .unwrap_or_else(|error| {
            error!("{}", SEPARATOR);
            error!("{}", error);
            error!("{}", SEPARATOR);
            process::exit(1);
        });

    let scenario: Scenario = match Scenario::new(config) {
        Ok(scenario) => scenario,
        Err(error) => {
            error!("{}", SEPARATOR);
            error!("Scenario initialization failed: {}", error);
            error!("{}", SEPARATOR);
            process::exit(1);
        }
    };

    let lifecycle = execution_lifecycle();

    match scenario.execute_with_lifecycle(lifecycle) {
        Ok(_) => {
            info!("{}", SEPARATOR);
            info!("{}", "Scenario completed successfully!".cyan());
            info!("{}", SEPARATOR);
        }
        Err(error) => {
            error!("{}", SEPARATOR);
            error!("Scenario execution failed: {}", error);
            error!("{}", SEPARATOR);
            process::exit(1);
        }
    }
}

fn execution_lifecycle() -> ExecutionLifecycle {
    let mut lifecycle = ExecutionLifecycle::default();
    lifecycle.steps = steps_lifecycle();
    lifecycle
}

fn steps_lifecycle() -> StepsLifecycle {
    let mut lifecycle = StepsLifecycle::default();
    lifecycle.before =
        |index: usize, task: &Task, total_steps: usize| {
            let step_number: usize = index + 1;
            let description = task.description();
            info!("{}", SEPARATOR);
            info!("{}", format!("[{step_number}/{total_steps}] {description}").purple());
        };
    lifecycle.remote_sudo = remote_sudo_lifecycle();
    lifecycle.sftp_copy = sftp_copy_lifecycle();
    lifecycle.rollback = rollback_lifecycle();
    lifecycle
}

fn remote_sudo_lifecycle() -> RemoteSudoLifecycle {
    let mut lifecycle = RemoteSudoLifecycle::default();
    lifecycle.before = |remote_sudo: &RemoteSudo| {
        info!("{}", "Executing:".yellow());
        info!("{}", &remote_sudo.command().bold());
    };
    lifecycle.channel_established = |channel: &mut dyn Read| {
        let mut output = String::new();
        if (*channel).read_to_string(&mut output).is_err() {
            warn!("{}", SEPARATOR);
            warn!("Channel output is not a valid UTF-8");
            warn!("{}", SEPARATOR);
            return;
        }
        let output = output.trim();
        info!("{}", output.chars().take(1000).collect::<String>().trim());
        if output.len() > 1000 {
            debug!("{}", output);
            info!("...output truncated...");
        }
    };
    lifecycle
}

fn sftp_copy_lifecycle() -> SftpCopyLifecycle {
    let mut lifecycle = SftpCopyLifecycle::default();
    lifecycle.before = |sftp_copy: &SftpCopy| {
        info!("{}", "Source:".yellow());
        info!("{}", &sftp_copy.source_path().bold());
        info!("{}", "Destination:".yellow());
        info!("{}", &sftp_copy.destination_path().bold());
    };
    lifecycle.files_ready =
        |source_file: &File, _, pb: &ProgressBar| {
            if let Ok(metadata) = source_file.metadata() {
                pb.set_length(metadata.len());
                pb.set_draw_target(ProgressDrawTarget::stderr());
                pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})").unwrap()
                    .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
                    .progress_chars("#>-"));
            } else {
                warn!("{}", SEPARATOR);
                warn!("Cannot query source file metadata");
                warn!("{}", SEPARATOR);
            }
        };
    lifecycle
}

fn rollback_lifecycle() -> RollbackLifecycle {
    let mut lifecycle = RollbackLifecycle::default();
    lifecycle.before =
        |rollback_steps: &RollbackSteps| {
            if rollback_steps.is_empty() {
                info!("{}", SEPARATOR);
                info!("[{}] No rollback actions found", "rollback".red());
            }
        };
    lifecycle.step = rollback_step_lifecycle();
    lifecycle
}

fn rollback_step_lifecycle() -> RollbackStepLifecycle {
    let mut lifecycle = RollbackStepLifecycle::default();
    lifecycle.before =
        |index: usize, rollback_task: &Task, total_rollback_steps: usize| {
            let task_number = index + 1;
            let description = rollback_task.description();
            info!("{}", SEPARATOR);
            info!("{}", format ! ("[{}] [{task_number}/{total_rollback_steps}] {}", "rollback".red(), description).purple());
        };
    lifecycle
}
