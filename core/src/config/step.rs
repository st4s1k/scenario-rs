use serde::Deserialize;

use super::on_fail::OnFailStepsConfig;

#[derive(Deserialize, Clone, Debug)]
pub struct StepConfig {
    pub task: String,
    pub on_fail: Option<OnFailStepsConfig>,
}
