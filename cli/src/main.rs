use chrono::Local;
use clap::Parser;
use core::data::Credentials;
use core::data::Scenario;
use core::data::Server;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    username: String,

    #[arg(short, long)]
    password: String,

    #[arg(long, default_value = "localhost")]
    host: String,

    #[arg(long, default_value = "22")]
    port: String,

    #[arg(short, long, value_name = "JSON_FILE")]
    scenario: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    jar: Option<PathBuf>,

}

fn main() {
    let cli: Cli = Cli::parse();

    let server: Server = Server::new(&cli.host, &cli.port);

    let username: &str = &cli.username;
    let password: &str = &cli.password;
    let credentials: Credentials = Credentials::new(username, password);

    let timestamp: String = Local::now().format("%Y-%m-%dT%H%M%S%:z").to_string();
    let local_jar_path: String = cli.jar.as_ref()
        .expect("Failed to get jar file path")
        .as_path()
        .as_os_str().to_os_string()
        .into_string()
        .expect("Failed to convert jar file path to string");
    let local_jar_basename: String = cli.jar.as_ref()
        .expect("Failed to get jar file path")
        .as_path()
        .file_name()
        .expect("Failed to get jar file basename")
        .to_os_string()
        .into_string()
        .expect("Failed to convert jar file basename to string");

    let additional_variables: HashMap<String, String> = HashMap::from([
        ("username".to_string(), username.to_string()),
        ("timestamp".to_string(), timestamp),
        ("local_jar_path".to_string(), local_jar_path),
        ("local_jar_basename".to_string(), local_jar_basename)
    ]);

    let scenario_file_path: PathBuf = cli.scenario
        .expect("Failed to get scenario file path");
    let scenario_file: File = File::open(scenario_file_path)
        .expect("Failed to open scenario file");
    let deploy_scenario: Scenario = Scenario::new(
        server,
        credentials,
        scenario_file,
        additional_variables,
    ).expect("Failed to init deploy scenario");

    deploy_scenario.execute().expect("Failed to deploy");
}
