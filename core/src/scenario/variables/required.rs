//! Defines required variables for scenarios.
//!
//! This module provides types and implementations for managing required variables
//! that are used within scenarios, including different variable types and their
//! transformations.

use chrono::Local;

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use crate::config::variables::required::{RequiredVariablesConfig, VariableTypeConfig};

/// A collection of required variables for a scenario.
///
/// This struct wraps a HashMap of variable names to `RequiredVariable` instances,
/// providing methods for managing these variables and handling derived variables.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::config::variables::required::{RequiredVariablesConfig, RequiredVariableConfig, VariableTypeConfig};
/// use scenario_rs_core::scenario::variables::required::RequiredVariables;
/// use std::collections::HashMap;
///
/// // Create a configuration with one string variable
/// let mut config_map = HashMap::new();
/// config_map.insert(
///     "username".to_string(),
///     RequiredVariableConfig {
///         label: Some("Username".to_string()),
///         var_type: VariableTypeConfig::String,
///         read_only: false,
///     }
/// );
/// let config = RequiredVariablesConfig::from(config_map);
///
/// // Create RequiredVariables from the config
/// let variables = RequiredVariables::from(&config);
///
/// // Access the variable
/// let username = variables.get("username").unwrap();
/// assert_eq!(username.label(), "Username");
/// ```
#[derive(Clone, Debug, Default)]
pub struct RequiredVariables(HashMap<String, RequiredVariable>);

impl Deref for RequiredVariables {
    type Target = HashMap<String, RequiredVariable>;

    /// Dereferences to the underlying HashMap for read operations.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RequiredVariables {
    /// Dereferences to the underlying HashMap for mutable operations.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&RequiredVariablesConfig> for RequiredVariables {
    /// Creates a `RequiredVariables` collection from a configuration.
    ///
    /// Initializes variables with appropriate types and default values based on
    /// the configuration. Timestamp variables are initialized with the current time.
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

impl RequiredVariables {
    /// Creates a map of variable names to their values.
    ///
    /// This is useful when you need only the values without the additional metadata.
    ///
    /// # Returns
    ///
    /// A HashMap where keys are variable names and values are the corresponding variable values.
    pub fn value_map(&self) -> HashMap<String, String> {
        self.iter()
            .map(|(key, var)| (key.clone(), var.value.clone()))
            .collect()
    }

    /// Updates existing variables with new values and adds derived variables.
    ///
    /// For path variables, automatically creates basename variables when the path
    /// points to a file. These basename variables are read-only and have the format
    /// "basename:{original_variable_name}".
    ///
    /// # Examples
    ///
    /// ```
    /// # use scenario_rs_core::config::variables::required::{RequiredVariablesConfig, RequiredVariableConfig, VariableTypeConfig};
    /// # use scenario_rs_core::scenario::variables::required::RequiredVariables;
    /// # use std::collections::HashMap;
    ///
    /// // Create a configuration with a path variable
    /// let mut config_map = HashMap::new();
    /// config_map.insert(
    ///     "file_path".to_string(),
    ///     RequiredVariableConfig {
    ///         label: Some("File Path".to_string()),
    ///         var_type: VariableTypeConfig::Path,
    ///         read_only: false,
    ///     }
    /// );
    /// let config = RequiredVariablesConfig::from(config_map);
    ///
    /// // Create RequiredVariables from the config
    /// let mut variables = RequiredVariables::from(&config);
    ///
    /// // Update the file path variable
    /// let mut updates = HashMap::new();
    /// updates.insert("file_path".to_string(), "/temp/data.txt".to_string());
    /// variables.upsert(updates);
    ///
    /// // A derived basename variable should be created
    /// assert!(variables.contains_key("basename:file_path"));
    /// assert_eq!(variables.get("basename:file_path").unwrap().value(), "data.txt");
    /// ```
    pub fn upsert(&mut self, variables: HashMap<String, String>) {
        let mut new_variables = HashMap::new();

        for (name, value) in variables {
            if let Some(required_variable) = self.get_mut(&name) {
                required_variable.value = value.clone();

                if let VariableType::Path = required_variable.var_type() {
                    let path = PathBuf::from(&value);

                    let is_dir_path = ends_with_directory_separator(&value);

                    if !is_dir_path && (path.is_file() || path.extension().is_some()) {
                        if let Some(file_name_str) = path.file_name().and_then(|s| s.to_str()) {
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

        for (key, var) in new_variables {
            self.insert(key, var);
        }
    }
}

fn ends_with_directory_separator(value: &str) -> bool {
    value
        .trim_end()
        .chars()
        .rev()
        .next()
        .map(|c| c == '\\' || std::path::is_separator(c))
        .unwrap_or(false)
}

/// Represents a single required variable with its metadata and value.
#[derive(Clone, Debug, Default)]
pub struct RequiredVariable {
    pub(crate) label: String,
    pub(crate) var_type: VariableType,
    pub(crate) value: String,
    pub(crate) read_only: bool,
}

impl Deref for RequiredVariable {
    type Target = String;

    /// Dereferences to the underlying value of the variable.
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl RequiredVariable {
    pub fn with_label(self, label: String) -> Self {
        RequiredVariable { label, ..self }
    }

    pub fn with_var_type(self, var_type: VariableType) -> Self {
        RequiredVariable { var_type, ..self }
    }

    pub fn with_value(self, value: String) -> Self {
        RequiredVariable { value, ..self }
    }

    pub fn with_read_only(self, read_only: bool) -> Self {
        RequiredVariable { read_only, ..self }
    }

    /// Returns the user-friendly label for this variable.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the current value of this variable.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the type of this variable.
    pub fn var_type(&self) -> &VariableType {
        &self.var_type
    }

    /// Returns whether this variable is read-only.
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub fn not_read_only(&self) -> bool {
        !self.read_only
    }
}

/// Defines the possible types for required variables.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum VariableType {
    /// A simple string variable.
    #[default]
    String,

    /// A file or directory path.
    /// Path variables can generate basename variables when they point to files.
    Path,

    /// A timestamp with a specific format.
    /// Initialized with the current time when created.
    Timestamp { format: String },
}

#[cfg(test)]
mod tests {
    use crate::{
        config::variables::required::{
            RequiredVariableConfig, RequiredVariablesConfig, VariableTypeConfig,
        },
        scenario::variables::required::{RequiredVariable, RequiredVariables, VariableType},
        utils::HasText,
    };
    use std::collections::HashMap;

    #[test]
    fn test_required_variable_getters() {
        // Given
        let variable = RequiredVariable {
            label: "Test Variable".to_string(),
            var_type: VariableType::String,
            value: "test_value".to_string(),
            read_only: true,
        };

        let timestamp_variable = RequiredVariable {
            label: "Date Variable".to_string(),
            var_type: VariableType::Timestamp {
                format: "%Y-%m-%d".to_string(),
            },
            value: "2023-05-15".to_string(),
            read_only: false,
        };

        // When & Then
        assert_eq!(variable.label(), "Test Variable");
        assert_eq!(variable.var_type(), &VariableType::String);
        assert_eq!(variable.value(), "test_value");
        assert!(variable.read_only());

        assert_eq!(timestamp_variable.label(), "Date Variable");
        assert_eq!(
            timestamp_variable.var_type(),
            &VariableType::Timestamp {
                format: "%Y-%m-%d".to_string()
            }
        );
        assert_eq!(timestamp_variable.value(), "2023-05-15");
        assert!(!timestamp_variable.read_only());
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
        variable.value = "new_value".to_string();

        // Then
        assert_eq!(variable.value(), "new_value");
    }

    #[test]
    fn test_from_config_with_all_variable_types() {
        // Given
        let mut config_map = HashMap::new();
        config_map.insert(
            "string_var".to_string(),
            RequiredVariableConfig {
                label: Some("String Variable".to_string()),
                var_type: VariableTypeConfig::String,
                read_only: true,
            },
        );
        config_map.insert(
            "path_var".to_string(),
            RequiredVariableConfig {
                label: Some("Path Variable".to_string()),
                var_type: VariableTypeConfig::Path,
                read_only: false,
            },
        );
        config_map.insert(
            "time_var".to_string(),
            RequiredVariableConfig {
                label: Some("Time Variable".to_string()),
                var_type: VariableTypeConfig::Timestamp {
                    format: "%H:%M:%S".to_string(),
                },
                read_only: false,
            },
        );
        config_map.insert(
            "unlabeled_var".to_string(),
            RequiredVariableConfig {
                label: None,
                var_type: VariableTypeConfig::String,
                read_only: false,
            },
        );
        let config = RequiredVariablesConfig::from(config_map);

        // When
        let required_vars = RequiredVariables::from(&config);

        // Then
        assert_eq!(required_vars.len(), 4);

        let string_var = required_vars.get("string_var").unwrap();
        assert_eq!(string_var.var_type(), &VariableType::String);
        assert_eq!(string_var.label(), "String Variable");
        assert!(string_var.read_only());
        assert_eq!(string_var.value(), "");

        let path_var = required_vars.get("path_var").unwrap();
        assert_eq!(path_var.var_type(), &VariableType::Path);
        assert_eq!(path_var.label(), "Path Variable");
        assert!(!path_var.read_only());
        assert_eq!(path_var.value(), "");

        let time_var = required_vars.get("time_var").unwrap();
        match time_var.var_type() {
            VariableType::Timestamp { format } => assert_eq!(format, "%H:%M:%S"),
            _ => panic!("Expected Timestamp variable type"),
        }
        assert_eq!(time_var.label(), "Time Variable");
        assert!(time_var.value().has_text());

        let unlabeled_var = required_vars.get("unlabeled_var").unwrap();
        assert_eq!(unlabeled_var.label(), "unlabeled_var");
    }

    #[test]
    fn test_timestamp_variable_initialization() {
        // Given
        let format = "%Y-%m-%d %H:%M";
        let config = RequiredVariableConfig {
            label: Some("Timestamp Test".to_string()),
            var_type: VariableTypeConfig::Timestamp {
                format: format.to_string(),
            },
            read_only: false,
        };
        let mut config_map = HashMap::new();
        config_map.insert("timestamp".to_string(), config);
        let required_config = RequiredVariablesConfig::from(config_map);

        // When
        let required_vars = RequiredVariables::from(&required_config);

        // Then
        let timestamp_var = required_vars.get("timestamp").unwrap();
        assert_eq!(timestamp_var.label(), "Timestamp Test");
        match timestamp_var.var_type() {
            VariableType::Timestamp { format: f } => assert_eq!(f, format),
            _ => panic!("Expected Timestamp variable type"),
        }

        // Verify the timestamp value matches the format (rough check)
        let value = timestamp_var.value();
        assert!(!value.is_empty());
        assert_eq!(value.len(), 16); // "%Y-%m-%d %H:%M" format has 16 characters
    }

    #[test]
    fn test_required_variables_default_and_empty_config() {
        // Given
        let empty_config = RequiredVariablesConfig::from(HashMap::new());

        // When
        let empty_vars = RequiredVariables::from(&empty_config);
        let default_vars = RequiredVariables::default();

        // Then
        assert!(empty_vars.is_empty());
        assert!(default_vars.is_empty());
    }

    #[test]
    fn test_required_variables_deref_and_deref_mut() {
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

        // When & Then (Deref test)
        assert_eq!(vars.len(), 1);
        assert!(vars.contains_key("var1"));
        let var = vars.get("var1").unwrap();
        assert_eq!(var.label(), "Label 1");
        assert_eq!(var.value(), "value1");

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

        let mut names = vars.keys().cloned().collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["var1", "var2"]);
    }

    #[test]
    fn test_required_variables_upsert() {
        // Given
        let mut vars = RequiredVariables::default();
        vars.insert(
            "string_var".to_string(),
            RequiredVariable {
                label: "String Var".to_string(),
                var_type: VariableType::String,
                value: "original".to_string(),
                read_only: false,
            },
        );
        vars.insert(
            "path_var".to_string(),
            RequiredVariable {
                label: "Path Variable".to_string(),
                var_type: VariableType::Path,
                value: "".to_string(),
                read_only: false,
            },
        );
        vars.insert(
            "directory_path".to_string(),
            RequiredVariable {
                label: "Directory Path".to_string(),
                var_type: VariableType::Path,
                value: "".to_string(),
                read_only: false,
            },
        );

        let mut update_map = HashMap::new();
        update_map.insert("string_var".to_string(), "updated".to_string());
        update_map.insert("path_var".to_string(), "/tmp/test/file.txt".to_string());
        update_map.insert("directory_path".to_string(), "/tmp/".to_string());
        update_map.insert("nonexistent".to_string(), "ignored".to_string());

        // When
        vars.upsert(update_map);

        // Then
        assert_eq!(vars.get("string_var").unwrap().value(), "updated");
        assert_eq!(vars.get("path_var").unwrap().value(), "/tmp/test/file.txt");
        assert_eq!(vars.get("directory_path").unwrap().value(), "/tmp/");

        assert!(vars.contains_key("basename:path_var"));
        let basename_var = vars.get("basename:path_var").unwrap();
        assert_eq!(basename_var.value(), "file.txt");
        assert_eq!(basename_var.label(), "Basename of Path Variable");
        assert!(basename_var.read_only());
        assert_eq!(basename_var.var_type(), &VariableType::String);

        assert!(!vars.contains_key("basename:directory_path"));
        assert!(!vars.contains_key("nonexistent"));
    }

    #[test]
    fn test_required_variables_upsert_multiple_paths() {
        // Given
        let mut vars = RequiredVariables::default();
        vars.insert(
            "path1".to_string(),
            RequiredVariable {
                label: "Path One".to_string(),
                var_type: VariableType::Path,
                value: "".to_string(),
                read_only: false,
            },
        );
        vars.insert(
            "path2".to_string(),
            RequiredVariable {
                label: "Path Two".to_string(),
                var_type: VariableType::Path,
                value: "".to_string(),
                read_only: false,
            },
        );

        let mut update_map = HashMap::new();
        update_map.insert("path1".to_string(), "/tmp/doc1.md".to_string());
        update_map.insert("path2".to_string(), "/var/log/file.log".to_string());

        // When
        vars.upsert(update_map);

        // Then
        assert_eq!(vars.len(), 4); // Original 2 + 2 basename variables
        assert_eq!(vars.get("path1").unwrap().value(), "/tmp/doc1.md");
        assert_eq!(vars.get("path2").unwrap().value(), "/var/log/file.log");
        assert_eq!(vars.get("basename:path1").unwrap().value(), "doc1.md");
        assert_eq!(vars.get("basename:path2").unwrap().value(), "file.log");
    }

    #[test]
    fn test_upsert_with_path_without_file() {
        // Given
        let mut vars = RequiredVariables::default();
        vars.insert(
            "path_var".to_string(),
            RequiredVariable {
                label: "Path Variable".to_string(),
                var_type: VariableType::Path,
                value: "".to_string(),
                read_only: false,
            },
        );

        let mut update_map = HashMap::new();
        update_map.insert("path_var".to_string(), "/tmp/directory/".to_string());

        // When
        vars.upsert(update_map);

        // Then
        assert_eq!(vars.len(), 1); // No basename was added
        assert_eq!(vars.get("path_var").unwrap().value(), "/tmp/directory/");
        assert!(!vars.contains_key("basename:path_var"));
    }

    #[test]
    fn test_upsert_with_windows_style_directory_terminators() {
        // Given
        let mut vars = RequiredVariables::default();
        vars.insert(
            "windows_dir".to_string(),
            RequiredVariable {
                label: "Windows Directory".to_string(),
                var_type: VariableType::Path,
                value: "".to_string(),
                read_only: false,
            },
        );
        vars.insert(
            "windows_dir_alt".to_string(),
            RequiredVariable {
                label: "Windows Directory Alt".to_string(),
                var_type: VariableType::Path,
                value: "".to_string(),
                read_only: false,
            },
        );

        let windows_dir_value = "C\\Temp\\";
        let windows_dir_alt_value = "C\\Temp/";

        let mut update_map = HashMap::new();
        update_map.insert("windows_dir".to_string(), windows_dir_value.to_string());
        update_map.insert("windows_dir_alt".to_string(), windows_dir_alt_value.to_string());

        // When
        vars.upsert(update_map);

        // Then
        assert_eq!(vars.get("windows_dir").unwrap().value(), windows_dir_value);
        assert_eq!(vars.get("windows_dir_alt").unwrap().value(), windows_dir_alt_value);
        assert!(!vars.contains_key("basename:windows_dir"));
        assert!(!vars.contains_key("basename:windows_dir_alt"));
    }

    #[test]
    fn test_variable_type_equality() {
        // Given
        let string_type = VariableType::String;
        let path_type = VariableType::Path;
        let timestamp_type1 = VariableType::Timestamp {
            format: "%Y-%m-%d".to_string(),
        };
        let timestamp_type2 = VariableType::Timestamp {
            format: "%Y-%m-%d".to_string(),
        };
        let timestamp_type3 = VariableType::Timestamp {
            format: "%H:%M:%S".to_string(),
        };

        // When & Then
        assert_eq!(string_type, VariableType::String);
        assert_ne!(string_type, path_type);
        assert_eq!(timestamp_type1, timestamp_type2);
        assert_ne!(timestamp_type1, timestamp_type3);
        assert_ne!(path_type, timestamp_type1);
    }
}
