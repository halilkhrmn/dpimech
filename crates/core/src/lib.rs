//! Shared types for the DPIMech service and GUI: the domain model,
//! persisted configuration, platform paths and the IPC protocol.

pub mod argpolicy;
pub mod args;
pub mod catalog;
pub mod config;
pub mod ipc;
pub mod lab;
pub mod model;
pub mod packages;
pub mod paths;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
