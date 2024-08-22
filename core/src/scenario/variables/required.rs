use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub struct RequiredVariables(HashMap<String, String>);

impl Deref for RequiredVariables {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
