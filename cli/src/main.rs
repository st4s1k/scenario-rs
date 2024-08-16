use chrono::Local;
use clap::Parser;
use core::data::Credentials;
use core::data::InternalVariables;
use core::data::Scenario;
use core::data::Server;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    service: String,

    #[arg(short, long)]
    username: String,

    #[arg(short, long)]
    password: String,

    #[arg(long, default_value = "localhost")]
    host: String,

    #[arg(long, default_value = "22")]
    port: String,

    #[arg(short, long, value_name = "JSON_FILE")]
    config: PathBuf,

    #[arg(long, value_name = "FILE")]
    jar: PathBuf,

}

fn main() {
    let cli: Cli = Cli::parse();

    let credentials = Credentials::new(cli.username, cli.password);

    let username: String = credentials.username().to_string();
    let timestamp: String = Local::now().format("%Y-%m-%dT%H%M%S%:z").to_string();
    let local_jar_path: String = cli.jar.as_path().as_os_str().to_os_string().into_string()
        .expect("Failed to convert jar file path to string");
    let local_jar_basename: String = cli.jar.as_path().file_name()
        .expect("Failed to get jar file basename")
        .to_os_string().into_string()
        .expect("Failed to convert jar file basename to string");

    let deploy_scenario: Scenario = Scenario::new(
        Server::new(&cli.host, &cli.port),
        credentials,
        cli.config,
        InternalVariables::new([
            ("service_name".to_string(), cli.service),
            ("username".to_string(), username),
            ("timestamp".to_string(), timestamp),
            ("local_jar_path".to_string(), local_jar_path),
            ("local_jar_basename".to_string(), local_jar_basename)
        ]),
    ).expect("Failed to init deploy scenario");

    deploy_scenario.execute().expect("Failed to deploy");
}
