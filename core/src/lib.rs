use crate::data::{Credentials, InternalVariables, InternalVariablesConfig, RemoteSudo, Scenario, ScenarioConfig, Server, SftpCopy, Step, Variables};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use regex::Regex;
use ssh2::{Channel, Session};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};
use tracing_subscriber::FmtSubscriber;

pub mod data;

const SEPARATOR: &'static str = "------------------------------------------------------------";

impl Scenario {
    pub fn new(
        server: Server,
        credentials: Credentials,
        scenario_file_path: PathBuf,
        internal_variables: InternalVariables,
    ) -> Result<Scenario> {
        let scenario_file: File = File::open(scenario_file_path)
            .expect("Failed to open scenario file");
        let config: ScenarioConfig = serde_json::from_reader(scenario_file)
            .with_context(|| "Failed to parse scenario file")?;

        internal_variables
            .validate(&config.variables.internal)
            .with_context(|| "Expected internal variables do not match the actual")?;

        let mut variables_map = HashMap::<String, String>::new();
        variables_map.extend(internal_variables.clone());
        variables_map.extend(config.variables.custom.clone());

        let variables = Variables(variables_map);
        let mut scenario = Scenario {
            server,
            credentials,
            variables,
            config,
        };

        scenario.resolve_placeholders()
            .with_context(|| "Failed to resolve placeholders in scenario")?;

        Ok(scenario)
    }

    fn resolve_placeholders(&mut self) -> Result<()> {
        let variables = &mut self.variables;
        variables.resolve_placeholders()
            .with_context(|| "Failed to resolve placeholders in variables")?;
        for step in &mut self.config.steps {
            step.resolve_placeholders(&variables)
                .with_context(|| "Failed to resolve placeholders in step")?;
        }
        Ok(())
    }

    pub fn execute(&self) -> Result<()> {
        let subscriber = FmtSubscriber::builder().finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global default subscriber");

        let session: &Session = &self.new_session()
            .with_context(|| "Failed to init ssh session")?;

        let total_steps: usize = (&self.config.steps).len();

        for (index, step) in self.config.steps.iter().enumerate() {
            let step_number: usize = index + 1;
            let description = step.description();
            info!("{}", SEPARATOR);
            info!("{}", format!("[{step_number}/{total_steps}] {description}").purple());
            if let Err(error) = self.execute_command(session, step) {
                error!("{}", SEPARATOR);
                error!("{}", error);
                error!("{}", SEPARATOR);
                return Err(error);
            }
        }

        let complete_message = self.config.complete_message.as_ref()
            .map(String::as_str)
            .unwrap_or("Scenario execution completed successfully!");

        info!("{}", SEPARATOR);
        info!("{}", complete_message.cyan());
        info!("{}", SEPARATOR);

        Ok(())
    }

    pub fn new_session(&self) -> Result<Session> {
        let remote_address = format!("{}:{}", &self.server.host, &self.server.port);
        let tcp = TcpStream::connect(&remote_address)
            .with_context(|| format!("Failed to connect to remote server: {remote_address}"))?;

        let mut session = Session::new()
            .with_context(|| "Failed to create a new session")?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .with_context(|| "Failed to initiate the SSH handshake")?;

        let username = &self.credentials.username;
        let password = &self.credentials.password;
        session.userauth_password(username, password)
            .with_context(|| "Failed to authenticate with password")?;

        Ok(session)
    }

    fn execute_command(
        &self,
        session: &Session,
        step: &Step,
    ) -> Result<()> {
        let error_message = step.error_message();
        let credentials = &self.credentials;

        let step_result = match step {
            Step::RemoteSudo { remote_sudo, .. } =>
                remote_sudo.execute(credentials, &session)
                    .with_context(|| format!("RemoteSudo: {error_message}")),
            Step::SftpCopy { sftp_copy, .. } =>
                sftp_copy.execute(&session)
                    .with_context(|| format!("SftpCopy: {error_message}"))
        };

        if let Err(error) = step_result {
            step.rollback(&credentials, &session)
                .with_context(|| format!("[{}] {error}", "rollback".red()))?;
            return Err(error);
        };

        Ok(())
    }
}

impl Step {
    fn rollback(
        &self,
        credentials: &Credentials,
        session: &Session,
    ) -> Result<()> {
        if let Some(rollback_steps) = self.rollback_steps() {
            for (index, rollback_step) in rollback_steps.iter().enumerate() {
                let step_number = index + 1;
                let total_rollback_steps = rollback_steps.len();
                let description = rollback_step.description();
                info!("{}", SEPARATOR);
                info!("{}", format!("[{}] [{step_number}/{total_rollback_steps}] {}", "rollback".red(), description).purple());
                match rollback_step {
                    Step::RemoteSudo { remote_sudo, .. } =>
                        remote_sudo.execute(&credentials, &session)
                            .with_context(|| format!("[{}]: {}", "rollback".red(), rollback_step.error_message()))?,
                    Step::SftpCopy { sftp_copy, .. } =>
                        sftp_copy.execute(&session)
                            .with_context(|| format!("[{}]: {}", "rollback".red(), rollback_step.error_message()))?
                }
            }
        } else {
            info!("[{}] No rollback actions found", "rollback".red());
        }
        Ok(())
    }

    fn resolve_placeholders(&mut self, variables: &Variables) -> Result<()> {
        match self {
            Step::RemoteSudo { remote_sudo, .. } =>
                remote_sudo.resolve_placeholders(variables)
                    .with_context(|| "Failed to resolve placeholders in RemoteSudo"),
            Step::SftpCopy { sftp_copy, .. } =>
                sftp_copy.resolve_placeholders(variables)
                    .with_context(|| "Failed to resolve placeholders in SftpCopy"),
        }
    }
}

impl RemoteSudo {
    fn execute(
        &self,
        credentials: &Credentials,
        session: &Session,
    ) -> Result<()> {
        info!("{}", "Executing:".yellow());

        let command = &self.command;
        info!("{}", command.bold());

        let password = &credentials.password;
        let mut channel: Channel = session.channel_session()
            .with_context(|| "Failed to create a new channel")?;
        channel.exec(&format!("echo {password} | sudo -S {command}"))
            .with_context(|| format!("Failed to execute remote command: {command}"))?;

        let mut output = String::new();
        channel.read_to_string(&mut output)
            .with_context(|| "Failed to read remote command output")?;
        let output = output.trim();
        info!("{}", output.chars().take(1000).collect::<String>().trim());
        if output.len() > 1000 {
            debug!("{}", output);
            info!("...output truncated...");
        }

        let exit_status = channel.exit_status()
            .with_context(|| format!("Failed to get exit status of remote command: {command}"))?;

        if exit_status != 0 {
            return Err(anyhow!("Remote command failed with status code {exit_status}"));
        }

        Ok(())
    }

    fn resolve_placeholders(&mut self, variables: &Variables) -> Result<()> {
        self.command = variables.resolve_placeholders_in(&self.command)
            .with_context(|| format!("Failed to resolve placeholders in: {}", self.command))?;
        Ok(())
    }
}

impl SftpCopy {
    fn execute(
        &self,
        session: &Session,
    ) -> Result<()> {
        info!("{}", "Source:".yellow());
        let source_path = &self.source_path;
        info!("{}", source_path.bold());

        info!("{}", "Destination:".yellow());
        let destination_path = &self.destination_path;
        info!("{}", destination_path.bold());

        let sftp = session.sftp()?;

        let mut source_file = File::open(&(&self.source_path))
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

    fn resolve_placeholders(&mut self, variables: &Variables) -> Result<()> {
        self.source_path = variables.resolve_placeholders_in(&self.source_path)
            .with_context(|| format!("Failed to resolve placeholders in: {}", self.source_path))?;
        self.destination_path = variables.resolve_placeholders_in(&self.destination_path)
            .with_context(|| format!("Failed to resolve placeholders in: {}", self.destination_path))?;
        Ok(())
    }
}

impl Variables {
    fn resolve_placeholders(&mut self) -> Result<()> {
        let mut iterations = 0;
        let max_iterations = 10;
        while iterations < max_iterations {
            let mut changes = false;
            for key in self.to_owned().keys().cloned() {
                let variables = &self;
                let value = &variables[&key];
                let new_value = self.resolve_placeholders_in(value)
                    .with_context(|| format!("Failed to resolve placeholders in: {value}"))?;
                if new_value != variables[&key] {
                    self.insert(key, new_value);
                    changes = true;
                }
            }
            if !changes {
                break;
            }
            iterations += 1;
        }
        for value in self.values() {
            value.assert_has_no_placeholders()
                .with_context(|| format!("Failed to resolve placeholders in: {value}"))?;
        }
        Ok(())
    }

    fn resolve_placeholders_in(&self, input: &str) -> Result<String> {
        let mut output = input.to_string();
        for (key, value) in self.deref() {
            output = output.replace(&format!("{{{key}}}"), value);
        }
        output.assert_has_no_placeholders()
            .with_context(|| format!("Failed to resolve placeholders in: {output}"))?;
        Ok(output)
    }
}

trait HasPlaceholders
where
    Self: AsRef<str>,
{
    fn assert_has_no_placeholders(&self) -> Result<()> {
        let placeholder_regex = Regex::new(r"\{\w+}")
            .with_context(|| "Failed to create placeholder regex")?;
        let value = self.as_ref();
        if placeholder_regex.find(value).is_some() {
            return Err(anyhow!("Unresolved placeholder found: {value}"));
        }
        Ok(())
    }
}

impl HasPlaceholders for String {}
impl HasPlaceholders for &str {}

impl InternalVariables {
    fn validate(&self, config: &InternalVariablesConfig) -> Result<()> {
        let undeclared_but_found =
            self.keys().into_iter()
                .filter(|var| !config.contains(var))
                .map(|var| var.to_string())
                .collect::<Vec<String>>();
        let declared_but_not_found =
            config.iter()
                .filter(|&var| !&self.contains_key(var))
                .map(|var| var.to_string())
                .collect::<Vec<String>>();

        if !undeclared_but_found.is_empty()
            || !declared_but_not_found.is_empty() {
            return Err(anyhow!("undeclared: {:?}, not found: {:?}", undeclared_but_found, declared_but_not_found));
        }

        Ok(())
    }
}