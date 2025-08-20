use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;

use crate::error::Result;
use crate::protocol::messages::StratumMessage;

/// Context passed through the middleware pipeline
#[derive(Debug, Clone)]
pub struct MessageContext {
    pub raw_message: String,
    pub parsed_message: Option<StratumMessage>,
    pub client_id: Option<String>,
    pub client_ip: Option<std::net::SocketAddr>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl MessageContext {
    pub fn new(raw_message: String) -> Self {
        Self {
            raw_message,
            parsed_message: None,
            client_id: None,
            client_ip: None,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_client_info(mut self, client_id: Option<String>, client_ip: Option<std::net::SocketAddr>) -> Self {
        self.client_id = client_id;
        self.client_ip = client_ip;
        self
    }

    pub fn with_parsed_message(mut self, message: StratumMessage) -> Self {
        self.parsed_message = Some(message);
        self
    }

    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Middleware trait for processing messages in the pipeline
#[async_trait]
pub trait Middleware: Send + Sync + Debug {
    async fn process(&self, context: MessageContext) -> Result<MessageContext>;
}

/// Pipeline for processing Stratum messages through middleware chain
#[derive(Debug, Clone)]
pub struct MessagePipeline {
    middleware: Vec<Arc<dyn Middleware>>,
}

impl MessagePipeline {
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    pub fn add_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.middleware.push(Arc::new(middleware));
        self
    }

    pub async fn process(&self, mut context: MessageContext) -> Result<MessageContext> {
        for middleware in &self.middleware {
            context = middleware.process(context).await?;
        }
        Ok(context)
    }

    pub fn middleware_count(&self) -> usize {
        self.middleware.len()
    }
}

impl Default for MessagePipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating message processing pipelines
#[derive(Debug)]
pub struct PipelineBuilder {
    pipeline: MessagePipeline,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            pipeline: MessagePipeline::new(),
        }
    }

    pub fn with_parsing(self) -> Self {
        self.add_middleware(crate::protocol::middleware::ParsingMiddleware::new())
    }

    pub fn with_validation(self) -> Self {
        self.add_middleware(crate::protocol::middleware::ValidationMiddleware::new())
    }

    pub fn with_rate_limiting(self, max_requests_per_minute: u64) -> Self {
        self.add_middleware(crate::protocol::middleware::RateLimitingMiddleware::new(max_requests_per_minute))
    }

    pub fn with_authentication(self) -> Self {
        self.add_middleware(crate::protocol::middleware::AuthenticationMiddleware::new())
    }

    pub fn with_logging(self) -> Self {
        self.add_middleware(crate::protocol::middleware::LoggingMiddleware::new())
    }

    pub fn add_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.pipeline = self.pipeline.add_middleware(middleware);
        self
    }

    pub fn build(self) -> MessagePipeline {
        self.pipeline
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}