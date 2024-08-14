#[allow(dead_code)]
pub mod deploy_rs {
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    pub struct Credentials {
        pub user: String,
        pub password: String,
    }

    #[derive(Deserialize)]
    pub struct Server {
        pub host: String,
        pub port: String,
    }

    #[derive(Deserialize, PartialEq)]
    pub enum Action {
        RemoteSudo,
        SftpCopy,
    }

    #[derive(Deserialize)]
    pub struct CommandConfig {
        pub action: Action,
        pub description: String,
        pub command: Option<String>,
        pub error_message: Option<String>,
        pub source_path: Option<String>,
        pub destination_path: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct Config {
        pub credentials: Credentials,
        pub server: Server,
        pub variables: HashMap<String, String>,
        pub commands: Vec<CommandConfig>,
    }
}
