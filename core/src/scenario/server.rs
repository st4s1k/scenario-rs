use crate::config::ServerConfig;

#[derive(Clone, Debug)]
pub struct Server {
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl From<&ServerConfig> for Server {
    fn from(server_config: &ServerConfig) -> Self {
        Server {
            host: server_config.host.clone(),
            port: server_config.port.unwrap_or(22),
        }
    }
}
