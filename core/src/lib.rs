//! Scenario-rs core library for automating server deployment scenarios.
//!
//! This library provides functionality to define, manage, and execute deployment
//! scenarios on remote servers. It supports running commands with sudo privileges,
//! copying files via SFTP, variable interpolation, and tracking execution progress.
//!
//! The library is organized into several key modules:
//! - `config`: Configuration structures for defining scenarios
//! - `scenario`: Core types and implementations for executing scenarios
//! - `session`: SSH session management and remote execution
//! - `trace`: Tracing and event capturing functionality

pub mod config;
pub mod scenario;
pub mod session;
pub mod trace;
pub mod utils;
