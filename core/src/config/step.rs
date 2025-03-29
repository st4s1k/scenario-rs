use serde::Deserialize;

use super::on_fail::OnFailStepsConfig;

#[derive(Clone, Debug, Deserialize)]
pub struct StepConfig {
    pub task: String,
    #[serde(rename = "on-fail")]
    pub on_fail: Option<OnFailStepsConfig>,
}
