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

impl Default for RequiredVariables {
    fn default() -> Self {
        RequiredVariables(Vec::new())
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
        let var1 = required_vars.iter().find(|v| v.name() == "var1").unwrap();
        let var2 = required_vars.iter().find(|v| v.name() == "var2").unwrap();
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
        let vars = RequiredVariables(vec![variable]);
        
        // Then
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name(), "direct_var");
        assert_eq!(vars[0].value(), "direct_value");
    }

    #[test]
    fn test_required_variables_deref() {
        // Given
        let vars = RequiredVariables(vec![
            RequiredVariable {
                name: "var1".to_string(),
                label: "Label 1".to_string(),
                value: "value1".to_string(),
            }
        ]);

        // When & Then
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name(), "var1");
    }

    #[test]
    fn test_required_variables_deref_mut() {
        // Given
        let mut vars = RequiredVariables(vec![
            RequiredVariable {
                name: "var1".to_string(),
                label: "Label 1".to_string(),
                value: "value1".to_string(),
            }
        ]);

        // When
        vars.push(RequiredVariable {
            name: "var2".to_string(),
            label: "Label 2".to_string(),
            value: "value2".to_string(),
        });

        // Then
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[1].name(), "var2");
    }

    #[test]
    fn test_required_variables_iteration() {
        // Given
        let vars = RequiredVariables(vec![
            RequiredVariable {
                name: "var1".to_string(),
                label: "Label 1".to_string(),
                value: "value1".to_string(),
            },
            RequiredVariable {
                name: "var2".to_string(),
                label: "Label 2".to_string(),
                value: "value2".to_string(),
            },
        ]);

        // When
        let mut names = Vec::new();
        for var in vars.iter() {
            names.push(var.name().to_string());
        }

        // Then
        assert_eq!(names, vec!["var1", "var2"]);
    }
}
