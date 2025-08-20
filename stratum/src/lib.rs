// New modular structure
pub mod config;
pub mod error;
pub mod network;
pub(crate) mod protocol;
pub(crate) mod services;
// storage: Future storage abstraction layer
pub(crate) mod utils;

// CLI interface
pub mod cli;

// Core modules
pub mod auth;
pub(crate) mod handler;
pub(crate) mod job;
pub(crate) mod listener;
pub mod manager;
pub mod processor;
pub(crate) mod submission;

// Public API exports
pub use config::Config;
pub use error::{ConfigError, Result, StratumError};
pub use listener::Listener;

// Internal use
pub use manager::Manager;
