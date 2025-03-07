use crate::{
    config::ScenarioConfig,
    scenario::{execute::Execute, tasks::Tasks},
};
use credentials::Credentials;
use errors::ScenarioError;
use events::Event;
use server::Server;
use ssh2::Session;
use std::{collections::HashMap, net::TcpStream, path::PathBuf, sync::mpsc::Sender};
use utils::SendEvent;
use variables::Variables;

pub mod credentials;
pub mod errors;
pub mod events;
pub mod execute;
pub mod on_fail;
pub mod remote_sudo;
pub mod server;
pub mod sftp_copy;
pub mod step;
pub mod steps;
pub mod task;
pub mod tasks;
pub mod utils;
pub mod variables;

#[derive(Debug)]
pub struct Scenario {
    pub(crate) server: Server,
    pub(crate) credentials: Credentials,
    pub(crate) execute: Execute,
    pub(crate) variables: Variables,
}

impl Scenario {
    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    pub fn variables_mut(&mut self) -> &mut Variables {
        &mut self.variables
    }
}

impl TryFrom<ScenarioConfig> for Scenario {
    type Error = ScenarioError;

    fn try_from(config: ScenarioConfig) -> Result<Self, Self::Error> {
        let server = Server::from(&config.server);
        let credentials = Credentials::from(&config.credentials);
        let tasks = Tasks::from(&config.tasks);
        let execute = Execute::try_from((&tasks, &config.execute))
            .map_err(ScenarioError::CannotCreateExecuteFromConfig)?;
        let mut variables = Variables::from(&config.variables);

        let mut username_vars = HashMap::new();
        username_vars.insert("username".to_string(), credentials.username.clone());
        variables.upsert(username_vars);

        let scenario = Scenario {
            server,
            credentials,
            execute,
            variables,
        };
        Ok(scenario)
    }
}

impl TryFrom<PathBuf> for Scenario {
    type Error = ScenarioError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let config = ScenarioConfig::try_from(path)
            .map_err(ScenarioError::CannotCreateScenarioFromConfig)?;
        Scenario::try_from(config)
    }
}

impl TryFrom<&str> for Scenario {
    type Error = ScenarioError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        let path = PathBuf::from(path);
        Scenario::try_from(path)
    }
}

impl Scenario {
    pub fn execute(&self, tx: Sender<Event>) -> Result<(), ScenarioError> {
        tx.send_event(Event::ScenarioStarted);

        #[cfg(debug_assertions)]
        {
            println!("[DEBUG] Starting scenario execution in debug mode");
            println!("[DEBUG] Server: {}:{}", self.server.host, self.server.port);
            println!("[DEBUG] Username: {}", self.credentials.username);
        }

        let session: Session = self.new_session()?;

        self.execute
            .steps
            .execute(&session, &self.variables, &tx)
            .map_err(ScenarioError::CannotExecuteSteps)?;

        tx.send_event(Event::ScenarioCompleted);
        Ok(())
    }

    pub fn new_session(&self) -> Result<Session, ScenarioError> {
        #[cfg(debug_assertions)]
        {
            println!(
                "[DEBUG] Creating mock session instead of connecting to {}:{}",
                self.server.host, self.server.port
            );

            let session = Session::new().map_err(ScenarioError::CannotCreateANewSession)?;
            return Ok(session);
        }

        #[cfg(not(debug_assertions))]
        {
            let host = &self.server.host;
            let port: &str = &self.server.port;
            let tcp = TcpStream::connect(&format!("{host}:{port}"))
                .map_err(ScenarioError::CannotConnectToRemoteServer)?;

            let mut session = Session::new().map_err(ScenarioError::CannotCreateANewSession)?;
            session.set_tcp_stream(tcp);
            session
                .handshake()
                .map_err(ScenarioError::CannotInitiateTheSshHandshake)?;

            let username = &self.credentials.username;

            match &self.credentials.password {
                Some(pwd) => session
                    .userauth_password(username, pwd)
                    .map_err(ScenarioError::CannotAuthenticateWithPassword)?,
                None => session
                    .userauth_agent(username)
                    .map_err(ScenarioError::CannotAuthenticateWithAgent)?,
            }

            Ok(session)
        }
    }
}
