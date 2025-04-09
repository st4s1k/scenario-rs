use serde::Deserialize;

/// Configuration for authentication credentials.
///
/// This struct defines the credentials used to authenticate with a remote server.
/// If password is not provided, SSH agent authentication will be attempted.
#[derive(Deserialize, Clone, Debug)]
pub struct CredentialsConfig {
    /// The username to authenticate as on the remote server
    pub username: String,
    /// Optional password for authentication (if not provided, SSH agent is used)
    pub password: Option<String>,
}
