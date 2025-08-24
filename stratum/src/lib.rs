//! # Loka Stratum Bitcoin Mining Proxy
//!
//! A high-performance Bitcoin Stratum V1 proxy server written in Rust, featuring:
//! - Lock-free optimizations for sub-microsecond metrics collection
//! - Comprehensive monitoring and alerting with Prometheus/Grafana integration
//! - SeaORM database integration for pool and miner management
//! - Advanced error handling and recovery mechanisms
//! - HTTP CONNECT tunneling support for flexible routing
//!
//! ## Architecture
//!
//! The proxy is built with a modular architecture:
//! - **Protocol Layer**: Handles Stratum V1 protocol parsing and validation
//! - **Network Layer**: Manages TCP connections and message routing
//! - **Service Layer**: Provides database, metrics, and caching services
//! - **Configuration**: TOML-based configuration with environment overrides
//! - **Error Handling**: Comprehensive error types with recovery strategies
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use loka_stratum::{Config, Listener};
//! use loka_stratum::services::database::DatabaseService;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize database service
//!     let database = Arc::new(DatabaseService::new("sqlite::memory:").await?);
//!
//!     // Load configuration from database
//!     let config = Config::load(&database).await?;
//!
//!     // Start listener (creates manager internally)
//!     let listener = Listener::new(config).await?;
//!     listener.accept().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - `sentry`: Enable Sentry error tracking integration

/// Core configuration management
pub mod config;

/// Comprehensive error handling with typed errors
pub mod error;

/// Network connection management and TCP proxy functionality
pub mod network;

/// Stratum V1 protocol implementation and message handling
pub mod protocol;

/// Sentry error tracking and monitoring integration
pub mod sentry;

/// Service layer: database, metrics, caching, and performance monitoring
pub mod services;

/// Utility functions for hashing, memory management, and time operations
pub mod utils;

/// Command-line interface for the stratum proxy
pub mod cli;

// === CORE MODULES ===

/// Authentication and authorization management
pub mod auth;

/// Internal connection handler (private module)
pub(crate) mod handler;

/// Mining job management and distribution
pub(crate) mod job;

/// TCP listener and connection management
pub(crate) mod listener;

/// Central coordinator for all subsystems
pub mod manager;

/// Message processing and transformation
pub mod processor;

/// Share submission tracking and validation
pub(crate) mod submission;

// === PUBLIC API EXPORTS ===

/// Configuration management for the stratum proxy.
///
/// Provides TOML-based configuration with environment variable overrides
/// and validation for all proxy settings.
pub use config::Config;

/// Error types and result handling for the stratum proxy.
///
/// Comprehensive error taxonomy with recovery strategies and detailed
/// error context for debugging and monitoring.
pub use error::{ConfigError, Result, StratumError};

/// TCP listener for accepting incoming miner connections.
///
/// Handles connection setup, protocol detection, and connection
/// lifecycle management with proper cleanup.
pub use listener::Listener;

/// Central manager coordinating all proxy subsystems.
///
/// Provides unified access to authentication, job management,
/// submissions, database services, and metrics collection.
pub use manager::Manager;
