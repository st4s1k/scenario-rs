//! Defines resolved variables for scenarios.
//!
//! This module provides types and implementations for managing resolved variables
//! that are used within scenarios

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// A fully-resolved snapshot of all variables (no remaining placeholders).
#[derive(Clone, Debug)]
pub struct ResolvedVariables(pub(crate) HashMap<String, String>);

impl Deref for ResolvedVariables {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ResolvedVariables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for ResolvedVariables {
    fn default() -> Self {
        ResolvedVariables(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use crate::scenario::variables::resolved::ResolvedVariables;
    use std::collections::HashMap;

    #[test]
    fn test_resolved_variables_default() {
        // Given & When
        let resolved = ResolvedVariables::default();

        // Then
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_resolved_variables_deref() {
        // Given
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        let resolved = ResolvedVariables(map);

        // When & Then
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_resolved_variables_deref_mut() {
        // Given
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        let mut resolved = ResolvedVariables(map);

        // When
        resolved.insert("key2".to_string(), "value2".to_string());
        resolved.remove("key1");

        // Then
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved.get("key2"), Some(&"value2".to_string()));
        assert_eq!(resolved.get("key1"), None);
    }

    #[test]
    fn test_resolved_variables_clone() {
        // Given
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        map.insert("key2".to_string(), "value2".to_string());
        let original = ResolvedVariables(map);

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(cloned.len(), original.len());
        assert_eq!(cloned.get("key1"), original.get("key1"));
        assert_eq!(cloned.get("key2"), original.get("key2"));
    }

    #[test]
    fn test_resolved_variables_debug() {
        // Given
        let mut map = HashMap::new();
        map.insert("debug_key".to_string(), "debug_value".to_string());
        let resolved = ResolvedVariables(map);

        // When
        let debug_string = format!("{:?}", resolved);

        // Then
        assert!(debug_string.contains("debug_key"));
        assert!(debug_string.contains("debug_value"));
    }

    #[test]
    fn test_resolved_variables_with_multiple_entries() {
        // Given
        let mut map = HashMap::new();
        map.insert("app_name".to_string(), "my-service".to_string());
        map.insert("env".to_string(), "production".to_string());
        map.insert(
            "log_dir".to_string(),
            "/var/log/my-service/production".to_string(),
        );

        // When
        let resolved = ResolvedVariables(map);

        // Then
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved.get("app_name"), Some(&"my-service".to_string()));
        assert_eq!(resolved.get("env"), Some(&"production".to_string()));
        assert_eq!(
            resolved.get("log_dir"),
            Some(&"/var/log/my-service/production".to_string())
        );
    }
}
