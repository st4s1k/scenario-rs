use crate::config::on_fail::OnFailStepsConfig;
use serde::Deserialize;

/// Configuration for a single execution step in a scenario.
///
/// A step represents a single task to be performed as part of a scenario,
/// along with optional fallback steps to execute if the task fails.
#[derive(Clone, Debug, Deserialize)]
pub struct StepConfig {
    /// The identifier of the task to execute in this step
    pub task: String,
    /// Optional steps to execute if this step's task fails
    #[serde(rename = "on-fail")]
    pub on_fail: Option<OnFailStepsConfig>,
}
