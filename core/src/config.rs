//! Configuration structures for scenario definitions.
//!
//! This module contains the configuration types that define how scenarios
//! are structured. These types are typically loaded from TOML configuration
//! files and used to initialize the runtime components in the scenario module.
//!
//! Key components include:
//! - `scenario`: Top-level scenario configuration
//! - `credentials`: Authentication settings for SSH connections
//! - `server`: Server connection details
//! - `execute`: Execution flow configuration
//! - `tasks`: Definitions of executable tasks
//! - `variables`: Variable declarations and configuration

pub mod credentials;
pub mod execute;
pub mod on_fail;
pub mod scenario;
pub mod server;
pub mod step;
pub mod steps;
pub mod task;
pub mod tasks;
pub mod variables;
