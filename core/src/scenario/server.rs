use crate::config::ServerConfig;

#[derive(Debug)]
pub struct Server {
    pub(crate) host: String,
    pub(crate) port: String,
}

impl From<&ServerConfig> for Server {
    fn from(server_config: &ServerConfig) -> Self {
        Server {
            host: server_config.host.clone(),
            port: server_config.port.as_ref()
                .map(String::clone)
                .unwrap_or("22".to_string()),
        }
    }
}
