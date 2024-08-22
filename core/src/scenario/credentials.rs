use crate::config::CredentialsConfig;

#[derive(Debug)]
pub struct Credentials {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
}

impl From<&CredentialsConfig> for Credentials {
    fn from(credentials_config: &CredentialsConfig) -> Self {
        Credentials {
            username: credentials_config.username.clone(),
            password: credentials_config.password.clone(),
        }
    }
}
