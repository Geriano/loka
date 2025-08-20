// Network layer - connection handling and management

pub mod connection;
pub mod handler;
// listener functionality is implemented in the main listener.rs
pub mod manager;
pub mod processors;
pub mod proxy;
pub mod tasks;

// Re-export key types
pub use connection::{Connection, ConnectionId, ConnectionState, DisconnectReason, ConnectionMetrics};
pub use handler::NetworkHandler;
pub use manager::{ConnectionManager, ConnectionManagerConfig, ConnectionManagerStats};
pub use processors::{CompositeProcessor, LoggingProcessor, LogLevel, StratumProcessor, RateLimitProcessor, MetricsProcessor};
pub use proxy::{BidirectionalProxy, MessageProcessor, PassThroughProcessor, ProxyBuilder};
pub use tasks::{TaskManager, TaskHandle, TaskId, TaskCategory, TaskMetadata, TaskManagerStatistics};