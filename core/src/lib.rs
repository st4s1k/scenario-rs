use crate::{
    data::{
        config::{
            RemoteSudoConfig,
            RequiredVariablesConfig,
            ScenarioConfig,
            SftpCopyConfig,
            StepConfig,
        },
        lifecycles::{
            ExecutionLifecycle,
            RemoteSudoLifecycle,
            RollbackLifecycle,
            SftpCopyLifecycle,
            StepLifecycle,
        },
        Credentials,
        RequiredVariables,
        Scenario,
        Server,
        Variables,
    },
    error::{
        PlaceholderResolutionError,
        RemoteSudoError,
        RequiredVariablesError,
        ScenarioError,
        SftpCopyError,
        StepError,
    },
};
use indicatif::ProgressBar;
use regex::Regex;
use ssh2::{Channel, Session};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::TcpStream,
    ops::Deref,
    path::Path,
};

pub mod data;
pub mod error;

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
        self.execute_with_lifecycle(ExecutionLifecycle::default())
    }

    pub fn execute_with_lifecycle(
        &self,
        mut lifecycle: ExecutionLifecycle,
    ) -> Result<(), ScenarioError> {
        let session: Session = self.new_session()?;

        (lifecycle.before)(&self);

        let steps = &self.config.steps;
        for (index, step) in steps.iter().enumerate() {
            (lifecycle.step.before)(index, step, &steps);
            self.execute_step(&session, step, &mut lifecycle.step)?;
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

    fn execute_step(
        &self,
        session: &Session,
        step_config: &StepConfig,
        lifecycle: &mut StepLifecycle,
    ) -> Result<(), ScenarioError> {
        let error_message = step_config.error_message().to_string();
        let credentials = &self.credentials;

        let step_result = match step_config {
            StepConfig::RemoteSudo { remote_sudo, .. } =>
                remote_sudo.execute(credentials, session, &mut lifecycle.remote_sudo)
                    .map_err(|error| ScenarioError::CannotExecuteRemoteSudoCommand(error, error_message)),
            StepConfig::SftpCopy { sftp_copy, .. } =>
                sftp_copy.execute(session, &mut lifecycle.sftp_copy)
                    .map_err(|error| ScenarioError::CannotExecuteSftpCopyCommand(error, error_message))
        };

        if let Err(error) = step_result {
            step_config.rollback(&credentials, session, &mut lifecycle.rollback)
                .map_err(ScenarioError::CannotRollbackStep)?;
            return Err(error);
        };

        Ok(())
    }
}

impl StepConfig {
    fn rollback(
        &self,
        credentials: &Credentials,
        session: &Session,
        lifecycle: &mut RollbackLifecycle,
    ) -> Result<(), StepError> {
        (lifecycle.before)(&self);
        if let Some(rollback_steps) = self.rollback_steps() {
            for (index, rollback_step) in rollback_steps.iter().enumerate() {
                (lifecycle.step.before)(index, rollback_step, rollback_steps);
                match rollback_step {
                    StepConfig::RemoteSudo { remote_sudo, .. } =>
                        remote_sudo.execute(&credentials, &session, &mut lifecycle.step.remote_sudo)
                            .map_err(StepError::CannotRollbackRemoteSudo)?,
                    StepConfig::SftpCopy { sftp_copy, .. } =>
                        sftp_copy.execute(&session, &mut lifecycle.step.sftp_copy)
                            .map_err(StepError::CannotRollbackSftpCopy)?
                }
            }
        }
        Ok(())
    }

    fn resolve_placeholders(&mut self, variables: &Variables) -> Result<(), StepError> {
        match self {
            StepConfig::RemoteSudo { remote_sudo, .. } =>
                remote_sudo.resolve_placeholders(variables)
                    .map_err(StepError::CannotResolveRemoteSudoPlaceholders),
            StepConfig::SftpCopy { sftp_copy, .. } =>
                sftp_copy.resolve_placeholders(variables)
                    .map_err(StepError::CannotResolveSftpCopyPlaceholders)
        }
    }
}

impl RemoteSudoConfig {
    fn execute(
        &self,
        credentials: &Credentials,
        session: &Session,
        lifecycle: &mut RemoteSudoLifecycle,
    ) -> Result<(), RemoteSudoError> {
        (lifecycle.before)(&self);

        let mut channel: Channel = session.channel_session()
            .map_err(RemoteSudoError::CannotEstablishSessionChannel)?;
        let password = &credentials.password;
        let command = &self.command;
        channel.exec(&format!("echo {password} | sudo -S {command}"))
            .map_err(RemoteSudoError::CannotExecuteRemoteCommand)?;

        (lifecycle.channel_established)(&mut channel);

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

impl SftpCopyConfig {
    fn execute(
        &self,
        session: &Session,
        lifecycle: &mut SftpCopyLifecycle,
    ) -> Result<(), SftpCopyError> {
        (lifecycle.before)(&self);

        let sftp = session.sftp()
            .map_err(SftpCopyError::CannotOpenChannelAndInitializeSftp)?;

        let mut source_file = File::open(&self.source_path)
            .map_err(SftpCopyError::CannotOpenSourceFile)?;
        let mut destination_file = sftp.create(Path::new(&self.destination_path))
            .map_err(SftpCopyError::CannotCreateDestinationFile)?;

        let pb = ProgressBar::hidden();

        (lifecycle.files_ready)(&source_file, &mut destination_file, &pb);

        let mut copy_buffer = Vec::new();

        source_file.read_to_end(&mut copy_buffer)
            .map_err(SftpCopyError::CannotReadSourceFile)?;

        pb.wrap_write(destination_file).write_all(&copy_buffer)
            .map_err(SftpCopyError::CannotWriteDestinationFile)?;

        pb.finish();

        (lifecycle.after)();

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
