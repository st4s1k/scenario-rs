use chrono::Local;

use crate::config::{RequiredVariablesConfig, VariableTypeConfig};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
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

        for (name, var_config) in config.iter() {
            let (var_type, value) = match &var_config.var_type {
                VariableTypeConfig::String => (VariableType::String, String::new()),
                VariableTypeConfig::Path => (VariableType::Path, String::new()),
                VariableTypeConfig::Timestamp { format } => (
                    VariableType::Timestamp {
                        format: format.clone(),
                    },
                    Local::now().format(format).to_string(),
                ),
            };

            required_variables.insert(
                name.clone(),
                RequiredVariable {
                    label: var_config.label.clone().unwrap_or_else(|| name.clone()),
                    var_type,
                    value,
                    read_only: var_config.read_only,
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
        let mut new_variables = HashMap::new();

        for (name, value) in variables {
            if self.contains_key(&name) {
                if let Some(required_variable) = self.get_mut(&name) {
                    required_variable.set_value(value.clone());

                    if let VariableType::Path = required_variable.var_type() {
                        let path = PathBuf::from(&value);
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                let basename_key = format!("basename:{}", name);
                                let label = format!("Basename of {}", required_variable.label());

                                new_variables.insert(
                                    basename_key,
                                    RequiredVariable {
                                        label,
                                        var_type: VariableType::String,
                                        value: file_name_str.to_string(),
                                        read_only: true,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        for (key, var) in new_variables {
            self.insert(key, var);
        }
    }
}

#[derive(Clone, Debug)]
pub struct RequiredVariable {
    pub(crate) label: String,
    pub(crate) var_type: VariableType,
    pub(crate) value: String,
    pub(crate) read_only: bool,
}

impl RequiredVariable {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: String) {
        self.value = value;
    }

    pub fn var_type(&self) -> &VariableType {
        &self.var_type
    }

    pub fn read_only(&self) -> bool {
        self.read_only
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableType {
    String,
    Path,
    Timestamp { format: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RequiredVariableConfig, RequiredVariablesConfig, VariableTypeConfig};
    use std::collections::HashMap;

    #[test]
    fn test_required_variable_getters() {
        // Given
        let variable = RequiredVariable {
            label: "Test Variable".to_string(),
            var_type: VariableType::String,
            value: "test_value".to_string(),
            read_only: false,
        };

        // When & Then
        assert_eq!(variable.label(), "Test Variable");
        assert_eq!(variable.var_type(), &VariableType::String);
        assert_eq!(variable.value(), "test_value");
    }

    #[test]
    fn test_required_variable_datetime_getters() {
        // Given
        let variable = RequiredVariable {
            label: "Date Variable".to_string(),
            var_type: VariableType::Timestamp {
                format: "%Y-%m-%d".to_string(),
            },
            value: "2023-05-15".to_string(),
            read_only: false,
        };

        // When & Then
        assert_eq!(variable.label(), "Date Variable");
        assert_eq!(
            variable.var_type(),
            &VariableType::Timestamp {
                format: "%Y-%m-%d".to_string()
            }
        );
        assert_eq!(variable.value(), "2023-05-15");
    }

    #[test]
    fn test_required_variable_set_value() {
        // Given
        let mut variable = RequiredVariable {
            label: "Test Variable".to_string(),
            var_type: VariableType::String,
            value: "initial_value".to_string(),
            read_only: false,
        };

        // When
        variable.set_value("new_value".to_string());

        // Then
        assert_eq!(variable.value(), "new_value");
    }

    #[test]
    fn test_required_variables_from_config() {
        // Given
        let mut config_map = HashMap::new();
        config_map.insert(
            "var1".to_string(),
            RequiredVariableConfig {
                label: Some("Variable 1".to_string()),
                var_type: VariableTypeConfig::String,
                read_only: false,
            },
        );
        config_map.insert(
            "var2".to_string(),
            RequiredVariableConfig {
                label: None,
                var_type: VariableTypeConfig::String,
                read_only: false,
            },
        );
        let config = RequiredVariablesConfig(config_map);

        // When
        let required_vars = RequiredVariables::from(&config);

        // Then
        assert_eq!(required_vars.len(), 2);
        let var1 = required_vars.get("var1").unwrap();
        let var2 = required_vars.get("var2").unwrap();
        assert_eq!(var1.label(), "Variable 1");
        assert_eq!(var1.var_type(), &VariableType::String);
        assert_eq!(var1.value(), "");
        assert_eq!(var2.label(), "var2");
        assert_eq!(var2.var_type(), &VariableType::String);
        assert_eq!(var2.value(), "");
    }

    #[test]
    fn test_datetime_variable() {
        // Given
        let mut config_map = HashMap::new();
        config_map.insert(
            "date".to_string(),
            RequiredVariableConfig {
                label: Some("Date".to_string()),
                var_type: VariableTypeConfig::Timestamp {
                    format: "%Y-%m-%d".to_string(),
                },
                read_only: false,
            },
        );
        let config = RequiredVariablesConfig(config_map);

        // When
        let required_vars = RequiredVariables::from(&config);

        // Then
        let date_var = required_vars.get("date").unwrap();
        assert_eq!(date_var.label(), "Date");
        assert_eq!(
            date_var.var_type(),
            &VariableType::Timestamp {
                format: "%Y-%m-%d".to_string()
            }
        );
        assert!(date_var.value().len() > 0);
    }

    #[test]
    fn test_required_variables_empty_config() {
        // Given
        let empty_config = RequiredVariablesConfig(HashMap::new());

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
            label: "Direct Variable".to_string(),
            var_type: VariableType::String,
            value: "direct_value".to_string(),
            read_only: false,
        };

        // When
        let mut vars = RequiredVariables::default();
        vars.insert("direct_var".to_string(), variable);

        // Then
        assert_eq!(vars.len(), 1);

        let var = vars.get("direct_var").unwrap();
        assert_eq!(var.label(), "Direct Variable");
        assert_eq!(var.var_type(), &VariableType::String);
        assert_eq!(var.value(), "direct_value");
    }

    #[test]
    fn test_required_variables_deref() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "var1".to_string(),
            RequiredVariable {
                label: "Label 1".to_string(),
                var_type: VariableType::String,
                value: "value1".to_string(),
                read_only: false,
            },
        );
        let vars = RequiredVariables(map);

        // When & Then
        assert_eq!(vars.len(), 1);
        assert!(vars.contains_key("var1"));

        let var = vars.get("var1").unwrap();
        assert_eq!(var.label(), "Label 1");
        assert_eq!(var.var_type(), &VariableType::String);
        assert_eq!(var.value(), "value1");
    }

    #[test]
    fn test_required_variables_deref_mut() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "var1".to_string(),
            RequiredVariable {
                label: "Label 1".to_string(),
                var_type: VariableType::String,
                value: "value1".to_string(),
                read_only: false,
            },
        );
        let mut vars = RequiredVariables(map);

        // When
        vars.insert(
            "var2".to_string(),
            RequiredVariable {
                label: "Label 2".to_string(),
                var_type: VariableType::String,
                value: "value2".to_string(),
                read_only: false,
            },
        );

        // Then
        assert_eq!(vars.len(), 2);
        assert!(vars.contains_key("var2"));
        let var = vars.get("var2").unwrap();

        assert_eq!(var.label(), "Label 2");
        assert_eq!(var.var_type(), &VariableType::String);
        assert_eq!(var.value(), "value2");
    }

    #[test]
    fn test_required_variables_iteration() {
        // Given
        let mut map = HashMap::new();
        map.insert(
            "var1".to_string(),
            RequiredVariable {
                label: "Label 1".to_string(),
                var_type: VariableType::String,
                value: "value1".to_string(),
                read_only: false,
            },
        );
        map.insert(
            "var2".to_string(),
            RequiredVariable {
                label: "Label 2".to_string(),
                var_type: VariableType::String,
                value: "value2".to_string(),
                read_only: false,
            },
        );
        let vars = RequiredVariables(map);

        // When
        let mut names = Vec::new();
        for (key, var) in vars.iter() {
            names.push(key.clone());
        }
        names.sort();

        // Then
        assert_eq!(names, vec!["var1", "var2"]);
    }
}
