use crate::{
    config::ExecuteConfig,
    scenario::{
        errors::ExecuteError,
        steps::Steps,
        tasks::Tasks,
    },
};

#[derive(Debug)]
pub struct Execute {
    pub(crate) steps: Steps,
}

impl Default for Execute {
    fn default() -> Self {
        Execute {
            steps: Steps::default(),
        }
    }
}

impl TryFrom<(&Tasks, &ExecuteConfig)> for Execute {
    type Error = ExecuteError;

    fn try_from((tasks, config): (&Tasks, &ExecuteConfig)) -> Result<Self, Self::Error> {
        let steps = Steps::try_from((tasks, &config.steps))
            .map_err(ExecuteError::CannotCreateStepsFromConfig)?;
        Ok(Execute { steps })
    }
}