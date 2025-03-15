use crate::config::RequiredVariablesConfig;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug)]
pub struct RequiredVariables(HashMap<String, RequiredVariable>);

impl Deref for RequiredVariables {
    type Target = HashMap<String, RequiredVariable>;

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
        let mut required_variables = HashMap::<String, RequiredVariable>::new();
        for (name, label) in config.deref() {
            required_variables.insert(
                name.clone(),
                RequiredVariable {
                    name: name.clone(),
                    label: label.clone(),
                    value: String::new(),
                },
            );
        }
        RequiredVariables(required_variables)
    }
}

impl Default for RequiredVariables {
    fn default() -> Self {
        RequiredVariables(HashMap::new())
    }
}

impl RequiredVariables {
    pub fn upsert(&mut self, variables: HashMap<String, String>) {
        for (name, value) in variables {
            if self.contains_key(&name) {
                if let Some(required_variable) = self.get_mut(&name) {
                    required_variable.set_value(value.clone());
                }
            }

            if name.starts_with("path:") {
                let path = PathBuf::from(value);
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        let basename_key = name.replace("path:", "basename:");

                        if self.contains_key(&basename_key) {
                            if let Some(required_variable) = self.get_mut(&basename_key) {
                                required_variable.set_value(file_name_str.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RequiredVariablesConfig;
    use std::collections::BTreeMap;

    #[test]
    fn test_required_variable_getters() {
        // Given
        let variable = RequiredVariable {
            name: "test_var".to_string(),
            label: "Test Variable".to_string(),
            value: "test_value".to_string(),
        };

        // When & Then
        assert_eq!(variable.name(), "test_var");
        assert_eq!(variable.label(), "Test Variable");
        assert_eq!(variable.value(), "test_value");
    }

    #[test]
    fn test_required_variable_set_value() {
        // Given
        let mut variable = RequiredVariable {
            name: "test_var".to_string(),
            label: "Test Variable".to_string(),
            value: "initial_value".to_string(),
        };

        // When
        variable.set_value("new_value".to_string());

        // Then
        assert_eq!(variable.value(), "new_value");
    }

    #[test]
    fn test_required_variables_from_config() {
        // Given
        let mut config_map = BTreeMap::new();
        config_map.insert("var1".to_string(), "Label 1".to_string());
        config_map.insert("var2".to_string(), "Label 2".to_string());
        let config = RequiredVariablesConfig(config_map);

        // When
        let required_vars = RequiredVariables::from(&config);

        // Then
        assert_eq!(required_vars.len(), 2);
        let var1 = required_vars.get("var1").unwrap();
        let var2 = required_vars.get("var2").unwrap();
        assert_eq!(var1.label(), "Label 1");
        assert_eq!(var1.value(), "");
        assert_eq!(var2.label(), "Label 2");
        assert_eq!(var2.value(), "");
    }

    #[test]
    fn test_required_variables_empty_config() {
        // Given
        let empty_config = RequiredVariablesConfig(BTreeMap::new());

        // When
        let empty_vars = RequiredVariables::from(&empty_config);

        // Then
        assert_eq!(empty_vars.len(), 0);
    }

    #[test]
    fn test_required_variables_default() {
        // Given & When
        let default_vars = RequiredVariables::default();

        // Then
        assert_eq!(default_vars.len(), 0);
    }

    #[test]
    fn test_required_variables_direct_construction() {
        // Given
        let variable = RequiredVariable {
            name: "direct_var".to_string(),
            label: "Direct Variable".to_string(),
            value: "direct_value".to_string(),
        };

        // When
        let mut vars = RequiredVariables::default();
        vars.insert(variable.name.clone(), variable);

        // Then
        assert_eq!(vars.len(), 1);

        let var = vars.get("direct_var").unwrap();
        assert_eq!(var.name(), "direct_var");
        assert_eq!(var.label(), "Direct Variable");
        assert_eq!(var.value(), "direct_value");
    }

    #[test]
    fn test_required_variables_deref() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "var1".to_string(),
            RequiredVariable {
                name: "var1".to_string(),
                label: "Label 1".to_string(),
                value: "value1".to_string(),
            },
        );
        let vars = RequiredVariables(map);

        // When & Then
        assert_eq!(vars.len(), 1);
        assert!(vars.contains_key("var1"));
        let var = vars.get("var1").unwrap();
        assert_eq!(var.name(), "var1");
    }

    #[test]
    fn test_required_variables_deref_mut() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "var1".to_string(),
            RequiredVariable {
                name: "var1".to_string(),
                label: "Label 1".to_string(),
                value: "value1".to_string(),
            },
        );
        let mut vars = RequiredVariables(map);

        // When
        vars.insert(
            "var2".to_string(),
            RequiredVariable {
                name: "var2".to_string(),
                label: "Label 2".to_string(),
                value: "value2".to_string(),
            },
        );

        // Then
        assert_eq!(vars.len(), 2);
        assert!(vars.contains_key("var2"));
        let var = vars.get("var2").unwrap();
        assert_eq!(var.name(), "var2");
    }

    #[test]
    fn test_required_variables_iteration() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "var1".to_string(),
            RequiredVariable {
                name: "var1".to_string(),
                label: "Label 1".to_string(),
                value: "value1".to_string(),
            },
        );
        map.insert(
            "var2".to_string(),
            RequiredVariable {
                name: "var2".to_string(),
                label: "Label 2".to_string(),
                value: "value2".to_string(),
            },
        );
        let vars = RequiredVariables(map);

        // When
        let mut names = Vec::new();
        for (key, var) in vars.iter() {
            names.push(key.clone());
            assert_eq!(key, var.name());
        }
        names.sort();

        // Then
        assert_eq!(names, vec!["var1", "var2"]);
    }
}
