use crate::{
    config::ExecuteConfig,
    scenario::steps::Steps,
};

pub struct Execute {
    pub(crate) steps: Steps,
}

impl From<&ExecuteConfig> for Execute {
    fn from(config: &ExecuteConfig) -> Self {
        let steps = Steps::from(&config.steps);
        Execute { steps }
    }
}