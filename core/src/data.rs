use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    pub fn new(user: &str, password: &str) -> Credentials {
        Credentials {
            username: user.to_string(),
            password: password.to_string(),
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Deserialize, Debug)]
pub struct Server {
    host: String,
    port: String,
}

impl Server {
    pub fn new(host: &str, port: &str) -> Server {
        Server {
            host: host.to_string(),
            port: port.to_string(),
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> &str {
        &self.port
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum Action {
    RemoteSudo,
    SftpCopy,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Step {
    action: Action,
    description: String,
    command: Option<String>,
    error_message: Option<String>,
    source_path: Option<String>,
    destination_path: Option<String>,
    rollback: Option<Vec<Step>>,
}

impl Step {
    pub fn action(&self) -> &Action {
        &self.action
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn set_description(&mut self, description: String) {
        self.description = description;
    }

    pub fn command(&self) -> Option<&String> {
        (&self).command.as_ref()
    }

    pub fn set_command(&mut self, command: String) {
        self.command = Some(command);
    }

    pub fn error_message(&self) -> Option<&String> {
        (&self).error_message.as_ref()
    }

    pub fn set_error_message(&mut self, error_message: String) {
        self.error_message = Some(error_message);
    }

    pub fn source_path(&self) -> Option<&String> {
        (&self).source_path.as_ref()
    }

    pub fn set_source_path(&mut self, source_path: String) {
        self.source_path = Some(source_path);
    }

    pub fn destination_path(&self) -> Option<&String> {
        (&self).destination_path.as_ref()
    }

    pub fn set_destination_path(&mut self, destination_path: String) {
        self.destination_path = Some(destination_path);
    }

    pub fn rollback(&self) -> Option<&Vec<Step>> {
        (&self).rollback.as_ref()
    }

    pub fn set_rollback(&mut self, rollback: Vec<Step>) {
        self.rollback = Some(rollback);
    }
}

#[derive(Deserialize, Debug)]
pub struct Scenario {
    credentials: Option<Credentials>,
    server: Option<Server>,
    variables: HashMap<String, String>,
    steps: Vec<Step>,
    complete_message: Option<String>,
}

impl Scenario {
    pub fn credentials(&self) -> Option<&Credentials> {
        (&self).credentials.as_ref()
    }

    pub fn set_credentials(&mut self, credentials: Credentials) {
        self.credentials = Some(credentials);
    }

    pub fn server(&self) -> Option<&Server> {
        (&self).server.as_ref()
    }

    pub fn set_server(&mut self, server: Server) {
        self.server = Some(server);
    }

    pub fn variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    pub fn add_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    pub fn steps(&self) -> &Vec<Step> {
        &self.steps
    }

    pub fn set_steps(&mut self, steps: Vec<Step>) {
        self.steps = steps;
    }

    pub fn complete_message(&self) -> Option<&String> {
        (&self).complete_message.as_ref()
    }
}
