use crate::{
    config::ScenarioConfig,
    scenario::{
        execute::Execute,
        tasks::Tasks,
    },
};
use credentials::Credentials;
use errors::ScenarioError;
use lifecycle::ExecutionLifecycle;
use server::Server;
use ssh2::Session;
use std::net::TcpStream;
use variables::Variables;

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
pub mod rollback;

#[derive(Debug)]
pub struct Scenario {
    pub(crate) server: Server,
    pub(crate) credentials: Credentials,
    pub(crate) execute: Execute,
    pub(crate) variables: Variables,
}

impl Scenario {
    pub fn variables(&mut self) -> &mut Variables {
        &mut self.variables
    }
}

impl Scenario {
    pub fn new(mut config: ScenarioConfig) -> Result<Scenario, ScenarioError> {
        let server = Server::from(&config.server);
        let credentials = Credentials::from(&config.credentials);
        config.variables.defined.insert("username".to_string(), credentials.username.clone());
        let tasks = Tasks::from(&config.tasks);
        let execute = Execute::try_from((&tasks, &config.execute))
            .map_err(ScenarioError::CannotCreateExecuteFromConfig)?;
        let variables = Variables::from(&config.variables);
        let scenario = Scenario {
            server,
            credentials,
            execute,
            variables,
        };
        Ok(scenario)
    }

    pub fn execute(&self) -> Result<(), ScenarioError> {
        self.execute_with_lifecycle(ExecutionLifecycle::default())
    }

    pub fn execute_with_lifecycle(
        &self,
        mut lifecycle: ExecutionLifecycle,
    ) -> Result<(), ScenarioError> {
        (lifecycle.before)(&self);

        let session: Session = self.new_session()?;

        self.execute.steps.execute(&session, &self.variables, &mut lifecycle.steps)
            .map_err(ScenarioError::CannotExecuteSteps)?;

        Ok(())
    }

    pub fn new_session(&self) -> Result<Session, ScenarioError> {
        let host = &self.server.host;
        let port: &str = &self.server.port;
        let tcp = TcpStream::connect(&format!("{host}:{port}"))
            .map_err(ScenarioError::CannotConnectToRemoteServer)?;

        let mut session = Session::new()
            .map_err(ScenarioError::CannotCreateANewSession)?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .map_err(ScenarioError::CannotInitiateTheSshHandshake)?;

        let username = &self.credentials.username;

        match &self.credentials.password {
            Some(pwd) => session.userauth_password(username, pwd)
                .map_err(ScenarioError::CannotAuthenticateWithPassword)?,
            None => session.userauth_agent(username)
                .map_err(ScenarioError::CannotAuthenticateWithAgent)?
        }

        Ok(session)
    }
}
