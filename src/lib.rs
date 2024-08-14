use anyhow::{anyhow, Context};
use chrono::Local;
use colored::Colorize;
use regex::Regex;
use ssh2::{Channel, Session};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use tracing::{debug, info};
use tracing_subscriber::FmtSubscriber;
use crate::data::deploy_rs::{Action, CommandConfig, Config};

mod data;

const SEPARATOR: &str = "------------------------------------------------------------";

pub fn init_tracing_subscriber() {
    let subscriber = FmtSubscriber::builder().finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");
}

pub fn init_config() -> anyhow::Result<Config> {
    let config_file_path = Path::new("config.json");
    let config_file = File::open(config_file_path)?;
    let mut config: Config = serde_json::from_reader(config_file)?;
    config.variables.insert("user".to_string(), config.credentials.user.clone());
    let timestamp = Local::now().format("%Y-%m-%dT%H%M%S%:z").to_string();
    config.variables.insert("timestamp".to_string(), timestamp);
    config.variables = replace_variables(config.variables)?;
    validate_config(&config)?;
    Ok(config)
}

pub fn validate_config(config: &Config) -> anyhow::Result<()> {
    for (index, command_config) in config.commands.iter().enumerate() {
        if command_config.action == Action::RemoteSudo && command_config.command.is_none() {
            return Err(anyhow!("Step {}: Missing field \"command\" for RemoteSudo action.", index + 1));
        }
        if command_config.action == Action::SftpCopy {
            if command_config.source_path.is_none() {
                return Err(anyhow!("Step {}: Missing field \"source_path\" for SftpCopy action.", index + 1));
            }
            if command_config.destination_path.is_none() {
                return Err(anyhow!("Step {}: Missing field \"destination_path\" for SftpCopy action.", index + 1));
            }
        }
    }
    Ok(())
}

pub fn init_session(config: &Config) -> anyhow::Result<Session> {
    let remote_address = format!("{}:{}", config.server.host, config.server.port);
    let tcp = TcpStream::connect(&remote_address)?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password(&config.credentials.user, &config.credentials.password)?;
    Ok(session)
}

pub fn deploy(config: Config, session: Session) -> anyhow::Result<()> {
    let total_steps: usize = config.commands.len();
    for (index, command_config) in config.commands.iter().enumerate() {
        let step_number: usize = index + 1;
        let Some(error_message) = &command_config.error_message else {
            return Err(anyhow!("required field \"error_message\""));
        };
        match command_config.action {
            Action::RemoteSudo =>
                if let Err(error) = remote_sudo(&config, &session, command_config, step_number, total_steps) {
                    return Err(anyhow!("RemoteSudo: {error_message}: {error}"));
                }
            Action::SftpCopy =>
                if let Err(error) = sftp_copy(&config, &session, command_config, step_number, total_steps) {
                    return Err(anyhow!("SftpCopy: {error_message}: {error}"))
                }
        }
    }

    info!("{}", SEPARATOR);
    info!("{}", "Deployment completed successfully!".cyan());
    info!("{}", SEPARATOR);

    Ok(())
}

fn remote_sudo(
    config: &Config,
    session: &Session,
    command_config: &CommandConfig,
    step_number: usize,
    total_steps: usize,
) -> anyhow::Result<()> {
    info!("{}", SEPARATOR);
    info!("{}", format!("[{step_number}/{total_steps}] {}", command_config.description).purple());
    info!("{}", "Executing:".yellow());

    let Some(command) = &command_config.command else {
        return Err(anyhow!("required field \"command\""));
    };
    let variables = &config.variables;
    let command = replace_placeholders(command, variables);
    info!("{}", command.bold());

    let mut channel: Channel = session.channel_session()?;
    channel.exec(&command).with_context(|| format!("Failed to execute remote command: {command}"))?;
    let mut output = String::new();
    channel.read_to_string(&mut output)?;

    info!("{}", output.chars().take(1000).collect::<String>());
    if output.len() > 1000 {
        debug!("{}", output);
        info!("...output truncated...");
    }

    let exit_status = channel.exit_status()?;

    if exit_status != 0 {
        return Err(anyhow!("Remote command failed with status code {}", exit_status));
    }

    Ok(())
}

fn sftp_copy(
    config: &Config,
    session: &Session,
    command_config: &CommandConfig,
    step_number: usize,
    total_steps: usize,
) -> anyhow::Result<()> {
    info!("{}", SEPARATOR);
    info!("{}", format!("[{step_number}/{total_steps}] {}", command_config.description).purple());

    let variables = &config.variables;

    let source_path = command_config.source_path.as_ref()
        .ok_or_else(|| anyhow!("Expected a source path for SftpCopy action"))?;
    let destination_path = command_config.destination_path.as_ref()
        .ok_or_else(|| anyhow!("Expected a destination path for SftpCopy action"))?;

    let source_path = replace_placeholders(source_path, variables);
    let destination_path = replace_placeholders(destination_path, variables);

    let sftp = session.sftp()?;
    let mut buffer = Vec::new();

    let mut source_file = File::open(&source_path)
        .with_context(|| format!("Failed to open source file: {}", source_path))?;
    let mut destination_file = sftp.create(Path::new(&destination_path))
        .with_context(|| format!("Failed to create destination file: {}", destination_path))?;

    source_file.read_to_end(&mut buffer)?;
    destination_file.write_all(&buffer)?;

    Ok(())
}

fn replace_variables(mut variables: HashMap<String, String>) -> anyhow::Result<HashMap<String, String>> {
    let mut iterations = 0;
    let max_iterations = 10;
    while iterations < max_iterations {
        let mut changes = false;
        for key in variables.keys().cloned().collect::<Vec<_>>() {
            let new_value = replace_placeholders(&variables[&key], &variables);
            if new_value != variables[&key] {
                variables.insert(key, new_value);
                changes = true;
            }
        }
        if !changes {
            break;
        }
        iterations += 1;
    }
    let placeholder_regex = Regex::new(r"\{\w+}")?;
    for value in variables.values() {
        if placeholder_regex.find(value).is_some() {
            return Err(anyhow!("Unresolved placeholder found: {value}"));
        }
    }
    Ok(variables)
}

fn replace_placeholders(input: &str, variables: &HashMap<String, String>) -> String {
    let mut output = input.to_string();
    for (key, value) in variables {
        output = output.replace(&format!("{{{key}}}"), value);
    }
    output
}
