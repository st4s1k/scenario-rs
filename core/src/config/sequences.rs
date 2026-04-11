use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

/// Named reusable sequences of task names.
#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub struct SequencesConfig(HashMap<String, Vec<String>>);

impl Deref for SequencesConfig {
    type Target = HashMap<String, Vec<String>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SequencesConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<String, Vec<String>>> for SequencesConfig {
    fn from(sequences: HashMap<String, Vec<String>>) -> Self {
        SequencesConfig(sequences)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequences_config_default() {
        let seq = SequencesConfig::default();
        assert!(seq.is_empty());
    }

    #[test]
    fn test_sequences_config_deserialization() {
        let toml_str = r#"
            cleanup = ["stop_service", "remove_files"]
            deploy = ["copy_config", "start_service"]
        "#;
        let seq: SequencesConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(
            seq.get("cleanup").unwrap(),
            &vec!["stop_service".to_string(), "remove_files".to_string()]
        );
    }

    #[test]
    fn test_sequences_config_deref_mut() {
        let mut seq = SequencesConfig::default();
        seq.insert(
            "new_seq".to_string(),
            vec!["task_x".to_string()],
        );
        assert_eq!(seq.len(), 1);
        assert!(seq.contains_key("new_seq"));
    }

    #[test]
    fn test_sequences_config_from_hashmap() {
        let mut map = HashMap::new();
        map.insert(
            "seq1".to_string(),
            vec!["task_a".to_string(), "task_b".to_string()],
        );
        let seq = SequencesConfig::from(map);
        assert_eq!(seq.len(), 1);
        assert_eq!(seq.get("seq1").unwrap().len(), 2);
    }
}
