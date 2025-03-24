use serde::Deserialize;

use super::steps::StepsConfig;

#[derive(Deserialize, Clone, Debug)]
pub struct ExecuteConfig {
    pub steps: StepsConfig,
}
