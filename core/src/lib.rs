use crate::data::{
    Credentials,
    RemoteSudo,
    RequiredVariables,
    RequiredVariablesConfig,
    Scenario,
    ScenarioConfig,
    Server,
    SftpCopy,
    Step,
    Variables,
};
use crate::error::{
    PlaceholderResolutionError,
    RemoteSudoError,
    RequiredVariablesError,
    ScenarioError,
    SftpCopyError,
    StepError,
};
use colored::Colorize;
use regex::Regex;
use ssh2::{Channel, Session};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::ops::Deref;
use std::path::Path;
use tracing::{debug, info};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::FmtSubscriber;

pub mod data;
pub mod error;

const SEPARATOR: &'static str = "------------------------------------------------------------";

impl Scenario {
    pub fn new(
        server: Server,
        credentials: Credentials,
        config: ScenarioConfig,
        required_variables: RequiredVariables,
    ) -> Result<Scenario, ScenarioError> {
        required_variables
            .validate(&config.variables.required)
            .map_err(ScenarioError::RequiredVariablesValidationFailed)?;

        let mut variables_map = HashMap::<String, String>::new();
        variables_map.extend(required_variables.clone());
        variables_map.extend(config.variables.defined.clone());

        let variables = Variables(variables_map);
        let mut scenario = Scenario {
            server,
            credentials,
            variables,
            config,
        };

        scenario.resolve_placeholders()?;

        Ok(scenario)
    }

    fn resolve_placeholders(&mut self) -> Result<(), ScenarioError> {
        let variables = &mut self.variables;
        variables.resolve_placeholders()
            .map_err(ScenarioError::CannotResolveVariablesPlaceholders)?;
        for step in &mut self.config.steps {
            step.resolve_placeholders(&variables)
                .map_err(ScenarioError::CannotResolveStepPlaceholders)?;
        }
        Ok(())
    }

    pub fn execute(&self) -> Result<(), ScenarioError> {
        let _tracing_guard =
            FmtSubscriber::builder()
                .compact()
                .without_time()
                .set_default();

        let session: &Session = &self.new_session()?;

        let total_steps: usize = (&self.config.steps).len();

        for (index, step) in self.config.steps.iter().enumerate() {
            let step_number: usize = index + 1;
            let description = step.description();
            info!("{}", format!("[{step_number}/{total_steps}] {description}").purple());
            self.execute_command(session, step)?;
        }

        Ok(())
    }

    pub fn new_session(&self) -> Result<Session, ScenarioError> {
        let remote_address = format!("{}:{}", &self.server.host, &self.server.port);
        let tcp = TcpStream::connect(&remote_address)
            .map_err(ScenarioError::CannotConnectToRemoteServer)?;

        let mut session = Session::new()
            .map_err(ScenarioError::CannotCreateANewSession)?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .map_err(ScenarioError::CannotInitiateTheSshHandshake)?;

        let username = &self.credentials.username;
        let password = &self.credentials.password;
        session.userauth_password(username, password)
            .map_err(ScenarioError::CannotAuthenticateWithPassword)?;

        Ok(session)
    }

    fn execute_command(
        &self,
        session: &Session,
        step: &Step,
    ) -> Result<(), ScenarioError> {
        let error_message = step.error_message().to_string();
        let credentials = &self.credentials;

        let step_result = match step {
            Step::RemoteSudo { remote_sudo, .. } =>
                remote_sudo.execute(credentials, &session)
                    .map_err(|error| ScenarioError::CannotExecuteRemoteSudoCommand(error, error_message)),
            Step::SftpCopy { sftp_copy, .. } =>
                sftp_copy.execute(&session)
                    .map_err(|error| ScenarioError::CannotExecuteSftpCopyCommand(error, error_message))
        };

        if let Err(error) = step_result {
            step.rollback(&credentials, &session)
                .map_err(ScenarioError::CannotRollbackStep)?;
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
    ) -> Result<(), StepError> {
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
                            .map_err(StepError::CannotRollbackRemoteSudo)?,
                    Step::SftpCopy { sftp_copy, .. } =>
                        sftp_copy.execute(&session)
                            .map_err(StepError::CannotRollbackSftpCopy)?
                }
            }
        } else {
            info!("[{}] No rollback actions found", "rollback".red());
        }
        Ok(())
    }

    fn resolve_placeholders(&mut self, variables: &Variables) -> Result<(), StepError> {
        match self {
            Step::RemoteSudo { remote_sudo, .. } =>
                remote_sudo.resolve_placeholders(variables)
                    .map_err(StepError::CannotResolveRemoteSudoPlaceholders),
            Step::SftpCopy { sftp_copy, .. } =>
                sftp_copy.resolve_placeholders(variables)
                    .map_err(StepError::CannotResolveSftpCopyPlaceholders)
        }
    }
}

impl RemoteSudo {
    fn execute(
        &self,
        credentials: &Credentials,
        session: &Session,
    ) -> Result<(), RemoteSudoError> {
        info!("{}", "Executing:".yellow());

        let command = &self.command;
        info!("{}", command.bold());

        let password = &credentials.password;
        let mut channel: Channel = session.channel_session()
            .map_err(RemoteSudoError::CannotEstablishSessionChannel)?;
        channel.exec(&format!("echo {password} | sudo -S {command}"))
            .map_err(RemoteSudoError::CannotExecuteRemoteCommand)?;

        let mut output = String::new();
        channel.read_to_string(&mut output)
            .map_err(RemoteSudoError::CannotReadRemoteCommandOutput)?;
        let output = output.trim();
        info!("{}", output.chars().take(1000).collect::<String>().trim());
        if output.len() > 1000 {
            debug!("{}", output);
            info!("...output truncated...");
        }

        let exit_status = channel.exit_status()
            .map_err(RemoteSudoError::CannotObtainRemoteCommandExitStatus)?;

        if exit_status != 0 {
            return Err(RemoteSudoError::RemoteCommandFailedWithStatusCode(exit_status));
        }

        Ok(())
    }

    fn resolve_placeholders(&mut self, variables: &Variables) -> Result<(), RemoteSudoError> {
        self.command = variables.resolve_placeholders_in(&self.command)
            .map_err(RemoteSudoError::CannotResolveCommandPlaceholders)?;
        Ok(())
    }
}

impl SftpCopy {
    fn execute(
        &self,
        session: &Session,
    ) -> Result<(), SftpCopyError> {
        info!("{}", "Source:".yellow());
        let source_path = &self.source_path;
        info!("{}", source_path.bold());

        info!("{}", "Destination:".yellow());
        let destination_path = &self.destination_path;
        info!("{}", destination_path.bold());

        let sftp = session.sftp()
            .map_err(SftpCopyError::CannotOpenChannelAndInitializeSftp)?;

        let mut source_file = File::open(&(&self.source_path))
            .map_err(SftpCopyError::CannotOpenSourceFile)?;
        let destination_file = sftp.create(Path::new(&destination_path))
            .map_err(SftpCopyError::CannotCreateDestinationFile)?;

        let metadata = source_file.metadata()
            .map_err(SftpCopyError::CannotQuerySourceMetadata)?;
        let pb = indicatif::ProgressBar::new(metadata.len());
        let mut destination_file = pb.wrap_write(destination_file);
        let mut buffer = Vec::new();

        source_file.read_to_end(&mut buffer)
            .map_err(SftpCopyError::CannotReadSourceFile)?;
        destination_file.write_all(&buffer)
            .map_err(SftpCopyError::CannotWriteDestinationFile)?;

        pb.finish_with_message("Copied source file to destination");

        Ok(())
    }

    fn resolve_placeholders(&mut self, variables: &Variables) -> Result<(), SftpCopyError> {
        self.source_path = variables.resolve_placeholders_in(&self.source_path)
            .map_err(SftpCopyError::CannotResolveSourcePathPlaceholders)?;
        self.destination_path = variables.resolve_placeholders_in(&self.destination_path)
            .map_err(SftpCopyError::CannotResolveDestinationPathPlaceholders)?;
        Ok(())
    }
}

impl Variables {
    fn resolve_placeholders(&mut self) -> Result<(), PlaceholderResolutionError> {
        let mut iterations = 0;
        let max_iterations = 10;
        while iterations < max_iterations {
            let mut changes = false;
            for key in self.to_owned().keys().cloned() {
                let variables = &self;
                let value = &variables[&key];
                let new_value = self.resolve_placeholders_in(value)?;
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

        let unresolved_keys = self.iter()
            .filter(|(_, value)| value.has_placeholders())
            .map(|(key, _)| key.to_owned())
            .collect::<Vec<String>>();

        if unresolved_keys.is_empty() {
            return Err(PlaceholderResolutionError::UnresolvedValues(unresolved_keys));
        }

        Ok(())
    }

    fn resolve_placeholders_in(&self, input: &str) -> Result<String, PlaceholderResolutionError> {
        let mut output = input.to_string();
        for (key, value) in self.deref() {
            output = output.replace(&format!("{{{key}}}"), value);
        }
        if output.has_placeholders() {
            return Err(PlaceholderResolutionError::UnresolvedValue(output));
        }
        Ok(output)
    }
}

trait HasPlaceholders
where
    Self: AsRef<str>,
{
    fn has_placeholders(&self) -> bool {
        let placeholder_regex = Regex::new(r"\{\w+}")
            .expect("`placeholder_regex` should be a valid regex");
        let value = self.as_ref();
        placeholder_regex.find(value).is_some()
    }
}

impl HasPlaceholders for String {}
impl HasPlaceholders for &str {}

impl RequiredVariables {
    fn validate(&self, config: &RequiredVariablesConfig) -> Result<(), RequiredVariablesError> {
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
            return Err(RequiredVariablesError::ValidationFailed(undeclared_but_found, declared_but_not_found));
        }

        Ok(())
    }
}
