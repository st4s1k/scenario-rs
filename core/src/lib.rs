use crate::data::{Action, Credentials, Scenario, Server, Step};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use regex::Regex;
use ssh2::{Channel, Session};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use tracing::{debug, error, info};
use tracing_subscriber::FmtSubscriber;

pub mod data;

impl Scenario {
    const SEPARATOR: &'static str = "------------------------------------------------------------";

    pub fn new(
        server: Server,
        credentials: Credentials,
        scenario_file: File,
        additional_variables: HashMap<String, String>,
    ) -> Result<Scenario> {
        let mut scenario: Scenario = serde_json::from_reader(scenario_file)
            .with_context(|| "Failed to parse scenario file")?;
        additional_variables.iter().for_each(|(key, value)| {
            scenario.add_variable(key.to_string(), value.to_string());
        });
        scenario.set_server(server);
        scenario.set_credentials(credentials);
        scenario.resolve_placeholders_in_variables()
            .with_context(|| "Failed to replace placeholders in variables")?;
        scenario.resolve_placeholders_in_steps()
            .with_context(|| "Failed to replace placeholders in steps")?;
        scenario.validate()
            .with_context(|| "Failed to validate scenario")?;
        Ok(scenario)
    }

    pub fn execute(&self) -> Result<()> {
        let subscriber = FmtSubscriber::builder().finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global default subscriber");

        let session: &Session = &self.new_session()
            .with_context(|| "Failed to init ssh session")?;

        let total_steps: usize = self.steps().len();

        for (index, step) in self.steps().iter().enumerate() {
            let step_number: usize = index + 1;
            if let Err(error) = self.execute_command(session, total_steps, step, step_number) {
                error!("{}", Self::SEPARATOR);
                error!("{}", error);
                error!("{}", Self::SEPARATOR);
                return Err(error);
            }
        }

        let complete_message = self.complete_message()
            .map(|msg| msg.as_str())
            .unwrap_or("Scenario execution completed successfully!");

        info!("{}", Self::SEPARATOR);
        info!("{}", complete_message.cyan());
        info!("{}", Self::SEPARATOR);

        Ok(())
    }

    fn validate(&self) -> Result<()> {
        for (index, step) in self.steps().iter().enumerate() {
            let step_number: usize = index + 1;
            if step.error_message().is_none() {
                return Err(anyhow!("Step {step_number}: Required field \"error_message\""));
            };
            if *step.action() == Action::RemoteSudo
                && step.command().is_none() {
                return Err(anyhow!("Step {step_number}: Missing field \"step\" for RemoteSudo action."));
            };
            if *step.action() == Action::SftpCopy {
                if step.source_path().is_none() {
                    return Err(anyhow!("Step {step_number}: Missing field \"source_path\" for SftpCopy action."));
                };
                if step.destination_path().is_none() {
                    return Err(anyhow!("Step {step_number}: Missing field \"destination_path\" for SftpCopy action."));
                };
            };
        }
        Ok(())
    }

    pub fn new_session(&self) -> Result<Session> {
        let server = self.server()
            .ok_or_else(|| anyhow!("Expected a server"))?;
        let remote_address = format!("{}:{}", server.host(), server.port());
        let tcp = TcpStream::connect(&remote_address)
            .with_context(|| format!("Failed to connect to remote server: {remote_address}"))?;
        let mut session = Session::new()
            .with_context(|| "Failed to create a new session")?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .with_context(|| "Failed to initiate the SSH handshake")?;
        let credentials = self.credentials()
            .ok_or_else(|| anyhow!("Expected credentials"))?;
        session.userauth_password(credentials.username(), credentials.password())
            .with_context(|| "Failed to authenticate with password")?;
        Ok(session)
    }

    fn execute_command(
        &self,
        session: &Session,
        total_steps: usize,
        step: &Step,
        step_number: usize,
    ) -> Result<()> {
        let error_message = step.error_message()
            .ok_or_else(|| anyhow!("Expected an error message"))?;
        match step.action() {
            Action::RemoteSudo => self.remote_sudo(&session, step, step_number, total_steps)
                .with_context(|| format!("RemoteSudo: {}", error_message))
                .or_else(|action_error| {
                    self.execute_rollback(&session, step)
                        .with_context(|| format!("[{}] RemoteSudo: {}", "rollback".red(), error_message))?;
                    Err(action_error)
                })?,
            Action::SftpCopy => self.sftp_copy(&session, step, step_number, total_steps)
                .with_context(|| format!("SftpCopy: {}", error_message))
                .or_else(|action_error| {
                    self.execute_rollback(&session, step)
                        .with_context(|| format!("[{}] SftpCopy: {}", "rollback".red(), error_message))?;
                    Err(action_error)
                })?
        }
        Ok(())
    }

    fn execute_rollback(
        &self,
        session: &Session,
        step: &Step,
    ) -> Result<()> {
        if let Some(rollback_commands) = step.rollback() {
            for (index, rollback_command) in rollback_commands.iter().enumerate() {
                let error_message = rollback_command.error_message()
                    .ok_or_else(|| anyhow!("[{}] Expected an error message", "rollback".red()))?;
                let step_number = index + 1;
                let total_rollback_steps = rollback_commands.len();
                info!("{}", format!("[{}] Executing rollback step {step_number}/{total_rollback_steps}", "rollback".red()).purple());
                match rollback_command.action() {
                    Action::RemoteSudo => self.remote_sudo(&session, rollback_command, step_number, total_rollback_steps)
                        .with_context(|| format!("[{}]: {}", "rollback".red(), error_message))?,
                    Action::SftpCopy => self.sftp_copy(&session, rollback_command, step_number, total_rollback_steps)
                        .with_context(|| format!("[{}]: {}", "rollback".red(), error_message))?,
                }
            }
        } else {
            info!("[{}] No rollback actions found", "rollback".red());
        }
        Ok(())
    }

    fn remote_sudo(
        &self,
        session: &Session,
        step: &Step,
        step_number: usize,
        total_steps: usize,
    ) -> Result<()> {
        info!("{}", Self::SEPARATOR);
        info!("{}", format!("[{step_number}/{total_steps}] {}", step.description()).purple());
        info!("{}", "Executing:".yellow());

        let command = step.command()
            .ok_or_else(|| anyhow!("Expected a command for RemoteSudo action"))?;
        info!("{}", command.bold());

        let credentials = self.credentials()
            .ok_or_else(|| anyhow!("Expected credentials for RemoteSudo action"))?;
        let password = credentials.password();
        let mut channel: Channel = session.channel_session()
            .with_context(|| "Failed to create a new channel")?;
        channel.exec(&format!("echo {password} | sudo -S {command}"))
            .with_context(|| format!("Failed to execute remote command: {command}"))?;

        Self::process_output(&mut channel)?;

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
        let output = output.trim();
        info!("{}", output.chars().take(1000).collect::<String>().trim());
        if output.len() > 1000 {
            debug!("{}", output);
            info!("...output truncated...");
        }
        Ok(())
    }

    fn sftp_copy(
        &self,
        session: &Session,
        step: &Step,
        step_number: usize,
        total_steps: usize,
    ) -> Result<()> {
        info!("{}", Self::SEPARATOR);
        info!("{}", format!("[{step_number}/{total_steps}] {}", step.description()).purple());

        info!("{}", "Source:".yellow());
        let source_path = step.source_path()
            .ok_or_else(|| anyhow!("Expected a source path for SftpCopy action"))?;
        info!("{}", source_path.bold());

        info!("{}", "Destination:".yellow());
        let destination_path = step.destination_path()
            .ok_or_else(|| anyhow!("Expected a destination path for SftpCopy action"))?;
        info!("{}", destination_path.bold());

        let sftp = session.sftp()?;

        let mut source_file = File::open(&source_path)
            .with_context(|| format!("Failed to open source file: {source_path}"))?;
        let destination_file = sftp.create(Path::new(&destination_path))
            .with_context(|| format!("Failed to create destination file: {destination_path}"))?;

        let pb = indicatif::ProgressBar::new(source_file.metadata()?.len());
        let mut destination_file = pb.wrap_write(destination_file);
        let mut buffer = Vec::new();

        source_file.read_to_end(&mut buffer)
            .with_context(|| format!("Failed to read source file: {source_path}"))?;
        destination_file.write_all(&buffer)
            .with_context(|| format!("Failed to write destination file: {destination_path}"))?;

        pb.finish_with_message("Copied source file to destination");

        Ok(())
    }

    fn resolve_placeholders_in_variables(&mut self) -> Result<()> {
        let keys = self.variables().keys().cloned().collect::<Vec<String>>();
        let mut iterations = 0;
        let max_iterations = 10;
        while iterations < max_iterations {
            let mut changes = false;
            for key in &keys {
                let variables = self.variables();
                let new_value = self.resolve_placeholders(&variables[key]);
                if new_value != variables[key] {
                    self.add_variable(key.to_string(), new_value);
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
        for value in self.variables().values() {
            if placeholder_regex.find(value).is_some() {
                return Err(anyhow!("Unresolved placeholder found: {value}"));
            }
        }
        Ok(())
    }

    fn resolve_placeholders_in_steps(&mut self) -> Result<()> {
        let complete_steps = self.resolve_placeholders_in_steps_recursively(self.steps());
        self.set_steps(complete_steps);
        Ok(())
    }

    fn resolve_placeholders_in_steps_recursively(&self, steps: &Vec<Step>) -> Vec<Step> {
        let complete_steps = &mut steps.to_vec();
        for step in &mut *complete_steps {
            step.set_description(self.resolve_placeholders(step.description()));
            step.command()
                .map(|command| self.resolve_placeholders(command))
                .map(|command| step.set_command(command));
            step.error_message()
                .map(|error_message| self.resolve_placeholders(error_message))
                .map(|error_message| step.set_error_message(error_message));
            step.source_path()
                .map(|source_path| self.resolve_placeholders(source_path))
                .map(|source_path| step.set_source_path(source_path));
            step.destination_path()
                .map(|destination_path| self.resolve_placeholders(destination_path))
                .map(|destination_path| step.set_destination_path(destination_path));
            if let Some(rollback) = step.rollback() {
                let complete_rollback_steps =
                    self.resolve_placeholders_in_steps_recursively(rollback);
                step.set_rollback(complete_rollback_steps);
            }
        }
        complete_steps.to_owned()
    }

    fn resolve_placeholders(&self, input: &str) -> String {
        let mut output = input.to_string();
        for (key, value) in self.variables() {
            output = output.replace(&format!("{{{key}}}"), value);
        }
        output
    }
}
