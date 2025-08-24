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
//! ```rust
//! use loka_stratum::protocol::handler::ProtocolHandler;
//! use std::sync::Arc;
//!
//! let handler = ProtocolHandler::new(manager, config, addr, None);
//! // Handler will automatically detect and route HTTP vs Stratum protocols
//! ```

pub mod http_handler;
pub mod stratum_handler;  
pub mod message_processor;
pub mod state_manager;

// Re-export the main ProtocolHandler for backward compatibility
pub use state_manager::ProtocolHandler;