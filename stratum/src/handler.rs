use std::net::SocketAddr;
use std::sync::Arc;

use crate::Config;
use crate::Manager;
use crate::protocol::handler::ProtocolHandler;

/// Factory for creating protocol handlers
pub struct Handler {
    manager: Arc<Manager>,
    config: Arc<Config>,
}

impl Handler {
    pub fn new(manager: Arc<Manager>, config: Arc<Config>) -> Self {
        Self { manager, config }
    }

    pub fn create(&self, addr: SocketAddr) -> ProtocolHandler {
        ProtocolHandler::new(Arc::clone(&self.manager), Arc::clone(&self.config), addr)
    }
}

impl Clone for Handler {
    fn clone(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
            config: Arc::clone(&self.config),
        }
    }
}
