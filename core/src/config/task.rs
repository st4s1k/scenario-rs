use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct TaskConfig {
    pub description: String,
    pub error_message: String,
    #[serde(flatten)]
    pub task_type: TaskType,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum TaskType {
    RemoteSudo {
        command: String,
    },
    SftpCopy {
        source_path: String,
        destination_path: String,
    },
}
