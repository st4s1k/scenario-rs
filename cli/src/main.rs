use crate::trace::ScenarioEventLayer;
use clap::Parser;
use scenario_rs::scenario::Scenario;
use std::{error::Error, path::PathBuf, process};
use tracing::{error, Level};
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry,
};

mod trace;

/// Command-line interface for the scenario runner.
///
/// This tool executes deployment scenarios defined in configuration files.
/// It supports setting required variables and controlling log verbosity.
#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    /// Path to the TOML file containing the scenario configuration
    #[arg(short, long, value_name = "TOML_FILE")]
    config_path: PathBuf,

    /// Log level for the application
    #[arg(short, long, value_name = "LOG_LEVEL", default_value_t = Level::INFO)]
    log_level: Level,

    /// Required variables in the format KEY=VALUE
    #[arg(short, long, value_name = "REQUIRED_VARIABLES", value_parser = parse_key_val::<String, String>)]
    required_variables: Vec<(String, String)>,
}

/// Application entry point.
///
/// Parses command-line arguments, initializes the scenario from the specified
/// configuration file, sets up tracing with custom event handling, applies any
/// required variables, and executes the scenario.
fn main() {
    let cli: Cli = Cli::parse();

    // Initialize the tracing system with the custom ScenarioEventLayer
    Registry::default()
        .with(LevelFilter::DEBUG)
        .with(
            fmt::Layer::new()
                .with_target(false)
                .compact()
                .with_filter(LevelFilter::from_level(cli.log_level)),
        )
        .with(ScenarioEventLayer::new())
        .init();

    // Load the scenario from the specified configuration file
    let mut scenario: Scenario = match Scenario::try_from(cli.config_path) {
        Ok(scenario) => scenario,
        Err(error) => {
            error!("Scenario initialization failed: {}", error);
            process::exit(1);
        }
    };

    // Apply any required variables provided via command-line arguments
    let required_variables = cli
        .required_variables
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();

    scenario
        .variables_mut()
        .required_mut()
        .upsert(required_variables);

    // Execute the scenario
    scenario.execute();
}

/// Parses a key-value pair from a string in the format "key=value".
///
/// This function is used by clap to convert command-line arguments into
/// typed key-value pairs for required variables.
///
/// # Arguments
///
/// * `s` - A string in the format "key=value"
///
/// # Returns
///
/// * `Ok((T, U))` with the parsed key and value if successful
/// * `Err` if the string doesn't contain '=' or the parsing fails
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}
