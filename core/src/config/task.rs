use serde::Deserialize;

/// Configuration for a task in a scenario.
///
/// A task is a single operation that can be performed as part of a scenario,
/// such as executing a command or copying a file.
#[derive(Deserialize, Clone, Debug)]
pub struct TaskConfig {
    /// Human-readable description of what this task does
    pub description: String,
    /// Error message to display if this task fails
    pub error_message: String,
    /// The specific type of task and its associated configuration
    #[serde(flatten)]
    pub task_type: TaskType,
}

/// Defines the different types of tasks that can be performed in a scenario.
///
/// Each variant corresponds to a specific operation with its own configuration parameters.
#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum TaskType {
    /// Execute a command with sudo privileges on a remote system.
    RemoteSudo {
        /// The command to execute
        command: String,
    },
    /// Copy a file from the local system to a remote system using SFTP.
    SftpCopy {
        /// Path to the source file on the local system
        source_path: String,
        /// Path to the destination on the remote system
        destination_path: String,
    },
}
