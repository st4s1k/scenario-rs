use std::ops::{Deref, DerefMut};

use serde::Deserialize;

use super::step::StepConfig;

#[derive(Deserialize, Clone, Debug)]
pub struct StepsConfig(pub Vec<StepConfig>);

impl Deref for StepsConfig {
    type Target = Vec<StepConfig>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StepsConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
