// Network layer - connection handling and management

pub mod connection;
pub mod handler;
// listener functionality is implemented in the main listener.rs
pub mod manager;
pub mod processors;
pub mod proxy;
pub mod tasks;

// Re-export key types
pub use connection::{
    Connection, ConnectionId, ConnectionMetrics, ConnectionState, DisconnectReason,
};
pub use handler::NetworkHandler;
pub use manager::{ConnectionManager, ConnectionManagerConfig, ConnectionManagerStats};
pub use processors::{
    CompositeProcessor, LogLevel, LoggingProcessor, MetricsProcessor, RateLimitProcessor,
    StratumProcessor,
};
pub use proxy::{BidirectionalProxy, MessageProcessor, PassThroughProcessor, ProxyBuilder};
pub use tasks::{
    TaskCategory, TaskHandle, TaskId, TaskManager, TaskManagerStatistics, TaskMetadata,
};
