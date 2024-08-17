use chrono::Local;
use clap::Parser;
use colored::Colorize;
use deploy_rs_core::{
    data::Credentials,
    data::RequiredVariables,
    data::Scenario,
    data::ScenarioConfig,
    data::Server,
};
use std::path::PathBuf;
use std::process::ExitCode;
use tracing::{error, info};
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
    let timestamp: String = Local::now().format("%Y-%m-%dT%H%M%S%:z").to_string();

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
        ("timestamp".to_string(), timestamp),
        ("local_jar_path".to_string(), local_jar_path),
        ("local_jar_basename".to_string(), local_jar_basename)
    ]);

    let deploy_scenario: Scenario = match Scenario::new(
        server,
        credentials,
        config,
        required_variables,
    ) {
        Ok(deploy_scenario) => deploy_scenario,
        Err(error) => {
            error!("{}", SEPARATOR);
            error!("Deployment scenario initialization failed: {}", error);
            error!("{}", SEPARATOR);
            return ExitCode::FAILURE;
        }
    };

    match deploy_scenario.execute() {
        Ok(_) => {
            info!("{}", SEPARATOR);
            info!("{}", "Deployment completed successfully!".cyan());
            info!("{}", SEPARATOR);
            ExitCode::SUCCESS
        }
        Err(error) => {
            error!("{}", SEPARATOR);
            error!("Deployment execution failed: {}", error);
            error!("{}", SEPARATOR);
            ExitCode::FAILURE
        }
    }
}
