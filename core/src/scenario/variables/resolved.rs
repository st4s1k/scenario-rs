//! Defines resolved variables for scenarios.
//!
//! This module provides types and implementations for managing resolved variables
//! that are used within scenarios

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// A collection of resolved variables for a scenario.
///
/// This struct wraps a HashMap of variable names to `ResolvedVariable` instances,
/// providing methods for managing these variables.
///
/// # Examples
///
/// Creating and accessing resolved variables:
///
/// ```
/// use std::collections::HashMap;
/// use scenario_rs_core::scenario::variables::resolved::ResolvedVariables;
/// 
/// // Create a collection of fully resolved variables
/// let mut resolved_vars = ResolvedVariables::default();
/// 
/// // Add some resolved variables
/// resolved_vars.insert("log_dir".to_string(), "/var/log/my-app".to_string());
/// resolved_vars.insert("config_path".to_string(), "/etc/my-app/config.json".to_string());
/// resolved_vars.insert("host".to_string(), "example.com".to_string());
/// 
/// // Access the resolved values
/// assert_eq!(resolved_vars.get("log_dir").unwrap(), "/var/log/my-app");
/// assert_eq!(resolved_vars.get("config_path").unwrap(), "/etc/my-app/config.json");
/// assert_eq!(resolved_vars.get("host").unwrap(), "example.com");
/// ```
///
/// Typically, `ResolvedVariables` are created by calling the `resolved()` method on a 
/// `Variables` instance, which automatically resolves all placeholders:
///
/// ```
/// use scenario_rs_core::scenario::variables::{Variables, defined::DefinedVariables};
/// use std::collections::HashMap;
/// 
/// // Create a default Variables instance
/// let mut variables = Variables::default();
/// 
/// // Add defined variables with nested references
/// let mut defined_vars = HashMap::new();
/// defined_vars.insert("app_name".to_string(), "my-service".to_string());
/// defined_vars.insert("env".to_string(), "production".to_string());
/// defined_vars.insert("log_dir".to_string(), "/var/log/{app_name}/{env}".to_string());
/// defined_vars.insert("config_path".to_string(), "/etc/{app_name}/config.{env}.json".to_string());
/// variables.defined_mut().extend(defined_vars);
/// 
/// // Resolve all variables
/// let resolved = variables.resolved().unwrap();
/// 
/// // Now all placeholders are resolved
/// assert_eq!(resolved.get("log_dir").unwrap(), "/var/log/my-service/production");
/// assert_eq!(resolved.get("config_path").unwrap(), "/etc/my-service/config.production.json");
/// ```
#[derive(Clone, Debug)]
pub struct ResolvedVariables(pub(crate) HashMap<String, String>);

impl Deref for ResolvedVariables {
    type Target = HashMap<String, String>;

    /// Dereferences to the underlying HashMap for read operations.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ResolvedVariables {
    /// Dereferences to the underlying HashMap for mutable operations.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for ResolvedVariables {
    /// Creates an empty collection of resolved variables.
    fn default() -> Self {
        ResolvedVariables(HashMap::new())
    }
}
