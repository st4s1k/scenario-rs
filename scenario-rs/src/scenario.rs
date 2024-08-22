use crate::{
    config::ScenarioConfig,
    scenario::{
        execute::Execute,
        step::Step,
        task::Task,
        tasks::Tasks,
    },
};
use credentials::Credentials;
use errors::ScenarioError;
use lifecycle::{
    ExecutionLifecycle,
    TaskLifecycle,
};
use server::Server;
use ssh2::Session;
use std::net::TcpStream;
use variables::{
    required::RequiredVariables,
    Variables,
};

pub mod credentials;
pub mod errors;
pub mod lifecycle;
pub mod server;
pub mod utils;
pub mod variables;
pub mod remote_sudo;
pub mod execute;
pub mod sftp_copy;
pub mod step;
pub mod steps;
pub mod task;
pub mod tasks;

pub struct Scenario {
    pub(crate) server: Server,
    pub(crate) credentials: Credentials,
    pub(crate) execute: Execute,
    pub(crate) tasks: Tasks,
}

impl Scenario {
    pub fn new(
        server: Server,
        credentials: Credentials,
        config: ScenarioConfig,
        required_variables: RequiredVariables,
    ) -> Result<Scenario, ScenarioError> {
        let execute = Execute::from(&config.execute);
        let variables = Variables::try_from((&required_variables, &config.variables))
            .map_err(ScenarioError::CannotCreateVariablesFromConfig)?;
        let tasks = Tasks::try_from((&config.tasks, &variables))
            .map_err(ScenarioError::CannotCreateTasksFromConfig)?;
        Ok(Scenario { server, credentials, execute, tasks })
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

        for (index, step) in self.execute.steps.iter().enumerate() {
            // TODO: Error handling - Step must be a valid task
            let task = self.tasks.get(&step.task).unwrap();
            (lifecycle.task.before)(index, task, &self.execute.steps);
            self.execute_step(&session, step, &mut lifecycle.task)?;
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
        step: &Step,
        lifecycle: &mut TaskLifecycle,
    ) -> Result<(), ScenarioError> {
        // TODO: Error handling - Step must be a valid task
        let task = &self.tasks.get(&step.task).unwrap();
        let error_message = task.error_message().to_string();
        let credentials = &self.credentials;

        let task_result = match task {
            Task::RemoteSudo { remote_sudo, .. } =>
                remote_sudo.execute(credentials, session, &mut lifecycle.remote_sudo)
                    .map_err(|error| ScenarioError::CannotExecuteRemoteSudoCommand(error, error_message)),
            Task::SftpCopy { sftp_copy, .. } =>
                sftp_copy.execute(session, &mut lifecycle.sftp_copy)
                    .map_err(|error| ScenarioError::CannotExecuteSftpCopyCommand(error, error_message))
        };

        if let Err(error) = task_result {
            step.rollback(&self.tasks, &credentials, session, &mut lifecycle.rollback)
                .map_err(ScenarioError::CannotRollbackTask)?;
            return Err(error);
        };

        Ok(())
    }
}