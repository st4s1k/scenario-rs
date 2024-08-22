use crate::{
    config::StepsConfig,
    scenario::step::Step,
};
use std::ops::{Deref, DerefMut};

pub struct Steps(Vec<Step>);

impl Deref for Steps {
    type Target = Vec<Step>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Steps {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&StepsConfig> for Steps {
    fn from(config: &StepsConfig) -> Self {
        let steps = config.deref().iter().map(Step::from).collect();
        Steps(steps)
    }
}
