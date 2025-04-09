use serde::Deserialize;

use super::steps::StepsConfig;

/// Configuration for scenario execution.
///
/// This struct defines the execution flow of a scenario,
/// including the sequence of steps to be performed.
#[derive(Deserialize, Clone, Debug)]
pub struct ExecuteConfig {
    /// The ordered sequence of steps to execute in the scenario
    pub steps: StepsConfig,
}
