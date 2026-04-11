//! Configuration structures for scenario definitions.
//!
//! These types are loaded from TOML configuration files and used to initialize
//! the runtime components in the scenario module.
//!
//! # Design Philosophy
//!
//! - **Hierarchical configuration**: Configs inherit from parents, common settings
//!   defined once and specialized in child configs.
//! - **Partial vs. Complete**: `PartialXConfig` supports inheritance/merging,
//!   `XConfig` has all required fields present.
//! - **Execution sequences**: Steps and on-fail sequences are defined as complete
//!   units in a single file for predictable behavior.
//! - **Variable interpolation**: Values can reference variables resolved at runtime.

pub mod credentials;
pub mod on_fail;
pub mod scenario;
pub mod sequences;
pub mod server;
pub mod step;
pub mod steps;
pub mod task;
pub mod tasks;
pub mod variables;

/// Returns the JSON Schema for the scenario configuration format.
pub fn json_schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::openapi3()
        .into_generator()
        .into_root_schema_for::<scenario::PartialScenarioConfig>()
}
