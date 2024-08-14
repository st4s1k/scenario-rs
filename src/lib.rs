use crate::data::deploy_rs::{Action, CommandConfig, DeployConfig};
use anyhow::{anyhow, Context, Result};
use chrono::Local;
use colored::Colorize;
use regex::Regex;
use ssh2::{Channel, Session};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use tracing::{debug, error, info};

mod data;

const SEPARATOR: &str = "------------------------------------------------------------";

pub fn init_config() -> Result<DeployConfig> {
    let config_file_path = Path::new("config.json");
    let config_file = File::open(config_file_path)
        .with_context(|| "Failed to open config file")?;
    let mut config: DeployConfig = serde_json::from_reader(config_file)
        .with_context(|| "Failed to parse config file")?;
    config.variables.insert("user".to_string(), config.credentials.user.clone());
    let timestamp = Local::now().format("%Y-%m-%dT%H%M%S%:z").to_string();
    config.variables.insert("timestamp".to_string(), timestamp);
    config.variables = replace_variables(config.variables)
        .with_context(|| "Failed to replace variables")?;
    validate_config(&config)
        .with_context(|| "Failed to validate config")?;
    Ok(config)
}

fn validate_config(config: &DeployConfig) -> Result<()> {
    for (index, command_config) in config.commands.iter().enumerate() {
        let step_number: usize = index + 1;
        if command_config.error_message.is_none() {
            return Err(anyhow!("Step {step_number}: Required field \"error_message\""));
        };
        if command_config.action == Action::RemoteSudo
            && command_config.command.is_none() {
            return Err(anyhow!("Step {step_number}: Missing field \"command\" for RemoteSudo action."));
        };
        if command_config.action == Action::SftpCopy {
            if command_config.source_path.is_none() {
                return Err(anyhow!("Step {step_number}: Missing field \"source_path\" for SftpCopy action."));
            };
            if command_config.destination_path.is_none() {
                return Err(anyhow!("Step {step_number}: Missing field \"destination_path\" for SftpCopy action."));
            };
        };
    }
    Ok(())
}

pub fn deploy(config: DeployConfig) -> Result<()> {
    let session: Session = init_session(&config)
        .with_context(|| "Failed to init ssh session")?;

    let total_steps: usize = config.commands.len();

    for (index, command_config) in config.commands.iter().enumerate() {
        let step_number: usize = index + 1;
        if let Err(error) = execute_command(&config, &session, total_steps, command_config, step_number) {
            error!("{}", SEPARATOR);
            error!("{}", error);
            error!("{}", SEPARATOR);
            return Err(error);
        }
    }

    info!("{}", SEPARATOR);
    info!("{}", "Deployment completed successfully!".cyan());
    info!("{}", SEPARATOR);

    Ok(())
}

pub fn init_session(config: &DeployConfig) -> Result<Session> {
    let remote_address = format!("{}:{}", config.server.host, config.server.port);
    let tcp = TcpStream::connect(&remote_address)
        .with_context(|| format!("Failed to connect to remote server: {remote_address}"))?;
    let mut session = Session::new()
        .with_context(|| "Failed to create a new session")?;
    session.set_tcp_stream(tcp);
    session.handshake()
        .with_context(|| "Failed to initiate the SSH handshake")?;
    session.userauth_password(&config.credentials.user, &config.credentials.password)
        .with_context(|| "Failed to authenticate with password")?;
    Ok(session)
}

fn execute_command(
    config: &DeployConfig,
    session: &Session,
    total_steps: usize,
    command_config: &CommandConfig,
    step_number: usize,
) -> Result<()> {
    let error_message = command_config.error_message.as_ref()
        .ok_or_else(|| anyhow!("Expected an error message"))?;
    match command_config.action {
        Action::RemoteSudo => remote_sudo(&config, &session, command_config, step_number, total_steps)
            .with_context(|| format!("RemoteSudo: {}", error_message))
            .or_else(|_| execute_rollback(&config, &session, command_config)
                .with_context(|| format!("[{}] RemoteSudo: {}", "rollback".red(), error_message)))?,
        Action::SftpCopy => sftp_copy(&config, &session, command_config, step_number, total_steps)
            .with_context(|| format!("SftpCopy: {}", error_message))
            .or_else(|_| execute_rollback(&config, &session, command_config)
                .with_context(|| format!("[{}] SftpCopy: {}", "rollback".red(), error_message)))?
    }
    Ok(())
}

fn execute_rollback(
    config: &DeployConfig,
    session: &Session,
    command_config: &CommandConfig,
) -> Result<()> {
    if let Some(rollback_commands) = &command_config.rollback {
        for (index, rollback_command) in rollback_commands.iter().enumerate() {
            let error_message = rollback_command.error_message.as_ref()
                .ok_or_else(|| anyhow!("[{}] Expected an error message", "rollback".red()))?;
            let step_number = index + 1;
            let total_rollback_steps = rollback_commands.len();
            info!("{}", format!("[{}] Executing rollback step {step_number}/{total_rollback_steps}", "rollback".red()).purple());
            match rollback_command.action {
                Action::RemoteSudo => remote_sudo(&config, &session, rollback_command, step_number, total_rollback_steps)
                    .with_context(|| format!("[{}]: {}", "rollback".red(), error_message))?,
                Action::SftpCopy => sftp_copy(&config, &session, rollback_command, step_number, total_rollback_steps)
                    .with_context(|| format!("[{}]: {}", "rollback".red(), error_message))?,
            }
        }
    } else {
        info!("[{}] No rollback actions found", "rollback".red());
    }
    Ok(())
}

fn remote_sudo(
    config: &DeployConfig,
    session: &Session,
    command_config: &CommandConfig,
    step_number: usize,
    total_steps: usize,
) -> Result<()> {
    info!("{}", SEPARATOR);
    info!("{}", format!("[{step_number}/{total_steps}] {}", command_config.description).purple());
    info!("{}", "Executing:".yellow());

    let variables = &config.variables;
    let command = command_config.command.as_ref()
        .ok_or_else(|| anyhow!("Expected a command for RemoteSudo action"))?;
    let command = replace_placeholders(command, variables);
    info!("{}", command.bold());

    let password = config.credentials.password.clone();
    let mut channel: Channel = session.channel_session()
        .with_context(|| "Failed to create a new channel")?;
    channel.exec(&format!("echo {password} | sudo -S {command}"))
        .with_context(|| format!("Failed to execute remote command: {command}"))?;

    process_output(&mut channel)?;

    let exit_status = channel.exit_status()
        .with_context(|| format!("Failed to get exit status of remote command: {command}"))?;

    if exit_status != 0 {
        return Err(anyhow!("Remote command failed with status code {exit_status}"));
    }

    Ok(())
}

fn process_output(channel: &mut Channel) -> Result<()> {
    let mut output = String::new();
    channel.read_to_string(&mut output)
        .with_context(|| "Failed to read remote command output")?;
    info!("{}", output.chars().take(1000).collect::<String>());
    if output.len() > 1000 {
        debug!("{}", output);
        info!("...output truncated...");
    }
    Ok(())
}

fn sftp_copy(
    config: &DeployConfig,
    session: &Session,
    command_config: &CommandConfig,
    step_number: usize,
    total_steps: usize,
) -> Result<()> {
    info!("{}", SEPARATOR);
    info!("{}", format!("[{step_number}/{total_steps}] {}", command_config.description).purple());

    let variables = &config.variables;

    info!("{}", "Source:".yellow());
    let source_path = command_config.source_path.as_ref()
        .ok_or_else(|| anyhow!("Expected a source path for SftpCopy action"))?;
    let source_path = replace_placeholders(source_path, variables);
    info!("{}", source_path.bold());

    info!("{}", "Destination:".yellow());
    let destination_path = command_config.destination_path.as_ref()
        .ok_or_else(|| anyhow!("Expected a destination path for SftpCopy action"))?;
    let destination_path = replace_placeholders(destination_path, variables);
    info!("{}", destination_path.bold());

    let sftp = session.sftp()?;
    let mut buffer = Vec::new();

    let mut source_file = File::open(&source_path)
        .with_context(|| format!("Failed to open source file: {source_path}"))?;
    let mut destination_file = sftp.create(Path::new(&destination_path))
        .with_context(|| format!("Failed to create destination file: {destination_path}"))?;

    source_file.read_to_end(&mut buffer)
        .with_context(|| format!("Failed to read source file: {source_path}"))?;
    destination_file.write_all(&buffer)
        .with_context(|| format!("Failed to write destination file: {destination_path}"))?;

    Ok(())
}

fn replace_variables(mut variables: HashMap<String, String>) -> Result<HashMap<String, String>> {
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
    let placeholder_regex = Regex::new(r"\{\w+}")
        .with_context(|| "Failed to create placeholder regex")?;
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
