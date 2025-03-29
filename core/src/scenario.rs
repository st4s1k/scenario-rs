use crate::{
    config::scenario::ScenarioConfig,
    scenario::{execute::Execute, tasks::Tasks},
    session::Session,
};
use credentials::Credentials;
use errors::ScenarioError;
use events::Event;
use server::Server;
use std::{path::PathBuf, sync::mpsc::Sender};
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

#[derive(Clone, Debug)]
pub struct Scenario {
    pub(crate) server: Server,
    pub(crate) credentials: Credentials,
    pub(crate) execute: Execute,
    pub(crate) variables: Variables,
    pub(crate) tasks: Tasks,
}

impl Scenario {
    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    pub fn variables_mut(&mut self) -> &mut Variables {
        &mut self.variables
    }

    pub fn tasks(&self) -> &Tasks {
        &self.tasks
    }
    
    pub fn steps(&self) -> &steps::Steps {
        &self.execute.steps
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

        // Insert the username into defined variables
        let mut variables_config = config.variables.clone();
        variables_config
            .defined
            .insert("username".to_string(), credentials.username.clone());

        let variables = Variables::from(&variables_config);

        let scenario = Scenario {
            server,
            credentials,
            execute,
            variables,
            tasks,
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
    pub fn execute(&self, tx: Sender<Event>) {
        if let Err(error) = self._execute(tx.clone()) {
            tx.send_event(Event::ScenarioError(format!("{error}")));
        }
    }

    pub fn _execute(&self, tx: Sender<Event>) -> Result<(), ScenarioError> {
        tx.send_event(Event::ScenarioStarted);

        let session = Session::new(&self.server, &self.credentials)
            .map_err(ScenarioError::CannotCreateANewSession)?;

        self.execute
            .steps
            .execute(&session, &self.variables, &tx)
            .map_err(ScenarioError::CannotExecuteSteps)?;

        tx.send_event(Event::ScenarioCompleted);
        Ok(())
    }
}
