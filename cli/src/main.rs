use clap::Parser;
use colored::Colorize;
use indicatif::{
    ProgressBar,
    ProgressDrawTarget,
    ProgressState,
    ProgressStyle,
};
use scenario_rs::scenario::steps::Steps;
use scenario_rs::{
    config::ScenarioConfig,
    scenario::{
        credentials::Credentials,
        lifecycle::{
            ExecutionLifecycle,
            RemoteSudoLifecycle,
            RollbackLifecycle,
            RollbackStepLifecycle,
            SftpCopyLifecycle,
            TaskLifecycle,
        },
        remote_sudo::RemoteSudo,
        server::Server,
        sftp_copy::SftpCopy,
        step::Step,
        task::Task,
        variables::required::RequiredVariables,
        Scenario,
    },
};
use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    process::ExitCode,
};
use tracing::{debug, error, info, warn};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    service_name: String,

    #[arg(short, long)]
    username: String,

    #[arg(short, long)]
    password: String,

    #[arg(long, default_value = "localhost")]
    host: String,

    #[arg(long, default_value = "22")]
    port: String,

    #[arg(short, long, value_name = "JSON_FILE")]
    config_path: PathBuf,

    #[arg(long, value_name = "FILE")]
    jar: PathBuf,

}

const SEPARATOR: &'static str = "------------------------------------------------------------";

fn main() -> ExitCode {
    let _tracing_guard = FmtSubscriber::builder().compact().without_time().init();

    let cli: Cli = Cli::parse();

    let server = Server::new(&cli.host, &cli.port);
    let credentials = Credentials::new(cli.username, cli.password);
    let config = match ScenarioConfig::try_from(cli.config_path) {
        Ok(config) => config,
        Err(error) => {
            error!("{}", SEPARATOR);
            error!("{}", error);
            error!("{}", SEPARATOR);
            return ExitCode::FAILURE;
        }
    };

    let username: String = credentials.username().to_string();

    let local_jar_path: String = match cli.jar.as_path().to_str() {
        Some(local_jar_path) => local_jar_path.to_string(),
        None => {
            let jar_path = cli.jar.to_str().unwrap_or("<not_a_valid_string>");
            error!("{}", SEPARATOR);
            error!("The JAR file path should be valid UTF-8: {}", jar_path);
            error!("{}", SEPARATOR);
            return ExitCode::FAILURE;
        }
    };

    let local_jar_basename: String = match cli.jar.as_path().file_name()
        .and_then(|file_name| file_name.to_str())
        .map(|file_name| file_name.to_string()) {
        Some(local_jar_basename) => local_jar_basename,
        None => {
            let jar_path = cli.jar.to_str().unwrap_or("<not_a_valid_string>");
            error!("{}", SEPARATOR);
            error!("The JAR file path is not a file: {}", jar_path);
            error!("{}", SEPARATOR);
            return ExitCode::FAILURE;
        }
    };

    let required_variables = RequiredVariables::new([
        ("service_name".to_string(), cli.service_name),
        ("username".to_string(), username),
        ("local_jar_path".to_string(), local_jar_path),
        ("local_jar_basename".to_string(), local_jar_basename)
    ]);

    let scenario: Scenario = match Scenario::new(
        server,
        credentials,
        config,
        required_variables,
    ) {
        Ok(scenario) => scenario,
        Err(error) => {
            error!("{}", SEPARATOR);
            error!("Scenario initialization failed: {}", error);
            error!("{}", SEPARATOR);
            return ExitCode::FAILURE;
        }
    };

    let lifecycle = execution_lifecycle();

    match scenario.execute_with_lifecycle(lifecycle) {
        Ok(_) => {
            info!("{}", SEPARATOR);
            info!("{}", "Scenario completed successfully!".cyan());
            info!("{}", SEPARATOR);
            ExitCode::SUCCESS
        }
        Err(error) => {
            error!("{}", SEPARATOR);
            error!("Scenario execution failed: {}", error);
            error!("{}", SEPARATOR);
            ExitCode::FAILURE
        }
    }
}

fn execution_lifecycle() -> ExecutionLifecycle {
    let mut lifecycle = ExecutionLifecycle::default();
    lifecycle.task = task_lifecycle();
    lifecycle
}

fn task_lifecycle() -> TaskLifecycle {
    let mut lifecycle = TaskLifecycle::default();
    lifecycle.before =
        |index: usize, task: &Task, steps: &Steps| {
            let task_number: usize = index + 1;
            let description = task.description();
            let total_steps: usize = steps.len();
            info!("{}", SEPARATOR);
            info!("{}", format!("[{task_number}/{total_steps}] {description}").purple());
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
        |step: &Step| {
            if step.rollback_steps().is_none() {
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
        |index: usize, rollback_task: &Task, rollback_steps: &Vec<String>| {
            let task_number = index + 1;
            let total_rollback_steps = rollback_steps.len();
            let description = rollback_task.description();
            info!("{}", SEPARATOR);
            info!("{}", format ! ("[{}] [{task_number}/{total_rollback_steps}] {}", "rollback".red(), description).purple());
        };
    lifecycle
}
