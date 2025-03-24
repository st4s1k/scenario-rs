use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: Option<u16>,
}
