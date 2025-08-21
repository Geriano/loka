// New modular structure
pub mod config;
pub mod error;
pub mod network;
pub mod protocol;
pub mod services;
// storage: Future storage abstraction layer
pub mod utils;

// CLI interface
pub mod cli;

// Mock pool for testing
#[cfg(feature = "mock-pool")]
pub mod mock;

// Mock miner for testing
#[cfg(feature = "mock-miner")]
pub mod miner;

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
