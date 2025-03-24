use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct CredentialsConfig {
    pub username: String,
    pub password: Option<String>,
}
