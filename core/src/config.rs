//! Configuration structures for scenario definitions.
//!
//! This module contains the configuration types that define how scenarios
//! are structured. These types are typically loaded from TOML configuration
//! files and used to initialize the runtime components in the scenario module.
//!
//! # Design Philosophy
//!
//! The configuration system follows several key design principles:
//!
//! - **Hierarchical configuration**: Configurations can inherit from parent
//!   configurations, allowing common settings to be defined once and specialized
//!   in child configurations.
//!
//! - **Partial vs. Complete configurations**: Many configuration components have both:
//!   - Partial versions (`PartialXConfig`) that support inheritance and merging
//!   - Complete versions (`XConfig`) with all required fields present
//!
//! - **Execution sequences**: Step sequences and on-fail sequences are intentionally
//!   defined as complete units in a single configuration file to ensure clear
//!   execution flow and predictable behavior.
//!
//! - **Variable interpolation**: Configuration values can reference variables
//!   which are resolved at runtime, supporting dynamic configuration.
//!
//! # Key Components
//!
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
