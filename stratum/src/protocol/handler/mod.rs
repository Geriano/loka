//! Protocol handler module providing focused components for different protocol types.
//!
//! This module breaks down the protocol handling functionality into specialized components:
//! - [`http_handler`] - HTTP request processing and CONNECT tunneling
//! - [`stratum_handler`] - Stratum V1 mining protocol implementation  
//! - [`message_processor`] - Message parsing, validation, and pipeline processing
//! - [`state_manager`] - Connection state management and lifecycle coordination
//!
//! # Architecture
//!
//! The protocol handler uses a pipeline architecture where messages flow through
//! multiple processing stages:
//!
//! ```text
//! Raw Message -> Parse -> Validate -> Authenticate -> Process -> Forward
//! ```
//!
//! Each component is responsible for a specific aspect of protocol handling,
//! making the system more maintainable and testable.
//!
//! # Examples
//!
//! ```rust,no_run
//! use loka_stratum::protocol::handler::ProtocolHandler;
//! use loka_stratum::{Manager, Config};
//! use loka_stratum::services::database::DatabaseService;
//! use std::sync::Arc;
//! use std::net::SocketAddr;
//!
//! # async fn example() -> loka_stratum::Result<()> {
//! let database = Arc::new(DatabaseService::new("sqlite::memory:").await?);
//! let config = Arc::new(Config::load(&database).await?);
//! let manager = Arc::new(Manager::new(config.clone(), database)?);
//! let addr: SocketAddr = "127.0.0.1:3333".parse().unwrap();
//! let handler = ProtocolHandler::new(manager, config, addr, None);
//! // Handler will automatically detect and route HTTP vs Stratum protocols
//! # Ok(())
//! # }
//! ```

pub mod http_handler;
pub mod message_processor;
pub mod state_manager;
pub mod stratum_handler;

// Re-export the main ProtocolHandler for backward compatibility
pub use state_manager::ProtocolHandler;
