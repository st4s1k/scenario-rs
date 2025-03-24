use std::ops::{Deref, DerefMut};

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct OnFailStepsConfig(pub(crate) Vec<String>);

impl Deref for OnFailStepsConfig {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OnFailStepsConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
