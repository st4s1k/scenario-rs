use crate::config::RequiredVariablesConfig;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct RequiredVariables(Vec<RequiredVariable>);

impl Deref for RequiredVariables {
    type Target = Vec<RequiredVariable>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&RequiredVariablesConfig> for RequiredVariables {
    fn from(config: &RequiredVariablesConfig) -> Self {
        let mut required_variables = Vec::<RequiredVariable>::new();
        for (name, label) in config.deref() {
            required_variables.push(RequiredVariable {
                name: name.clone(),
                label: label.clone(),
                value: String::new(),
            });
        }
        RequiredVariables(required_variables)
    }
}

#[derive(Debug)]
pub struct RequiredVariable {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) value: String,
}

impl RequiredVariable {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: String) {
        self.value = value;
    }
}
