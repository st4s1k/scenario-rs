use defined::DefinedVariablesConfig;
use required::RequiredVariablesConfig;
use serde::Deserialize;

use crate::scenario::errors::ScenarioConfigError;

pub mod defined;
pub mod required;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct VariablesConfig {
    #[serde(default)]
    pub required: RequiredVariablesConfig,
    #[serde(default)]
    pub defined: DefinedVariablesConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PartialVariablesConfig {
    pub required: Option<RequiredVariablesConfig>,
    pub defined: Option<DefinedVariablesConfig>,
}

impl PartialVariablesConfig {
    pub fn merge(&self, other: &PartialVariablesConfig) -> PartialVariablesConfig {
        let mut merged_required = match (&self.required, &other.required) {
            (Some(self_req), Some(other_req)) => self_req.merge(other_req),
            (None, Some(req)) => req.clone(),
            (Some(req), None) => req.clone(),
            (None, None) => RequiredVariablesConfig::default(),
        };

        let merged_defined = match (&self.defined, &other.defined) {
            (Some(self_defined), Some(other_defined)) => self_defined.merge(other_defined),
            (None, Some(defined)) => defined.clone(),
            (Some(defined), None) => defined.clone(),
            (None, None) => DefinedVariablesConfig::default(),
        };

        for key in merged_defined.keys() {
            merged_required.remove(key);
        }

        PartialVariablesConfig {
            required: Some(merged_required),
            defined: Some(merged_defined),
        }
    }
}

impl TryFrom<PartialVariablesConfig> for VariablesConfig {
    type Error = ScenarioConfigError;

    fn try_from(partial: PartialVariablesConfig) -> Result<Self, Self::Error> {
        Ok(VariablesConfig {
            required: partial.required.unwrap_or_default(),
            defined: partial.defined.unwrap_or_default(),
        })
    }
}
