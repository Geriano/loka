use crate::error::Result;
use crate::protocol::pipeline::{MessageContext, PipelineBuilder};

/// Example demonstrating how to create and use a message processing pipeline
pub async fn example_pipeline_usage() -> Result<()> {
    // Create a pipeline with multiple middleware layers
    let pipeline = PipelineBuilder::new()
        .with_logging()           // Log all messages
        .with_parsing()           // Parse raw messages into StratumMessage
        .with_validation()        // Validate parsed messages
        .with_rate_limiting(60)   // Allow max 60 requests per minute
        .with_authentication()    // Handle authentication
        .build();

    // Example raw Stratum message
    let raw_message = r#"{"id":1,"method":"mining.authorize","params":["user.worker","password"]}"#;
    
    // Create context for the message
    let context = MessageContext::new(raw_message.to_string())
        .with_client_info(
            Some("client123".to_string()),
            Some("127.0.0.1:8080".parse().unwrap())
        );

    // Process the message through the pipeline
    match pipeline.process(context).await {
        Ok(processed_context) => {
            tracing::info!("Message processed successfully");
            
            if let Some(ref parsed_message) = processed_context.parsed_message {
                tracing::info!("Parsed message type: {}", parsed_message.message_type());
            }
            
            if processed_context.get_metadata("authenticated").is_some() {
                tracing::info!("Client was authenticated");
            }
        }
        Err(e) => {
            tracing::error!("Failed to process message: {}", e);
        }
    }

    Ok(())
}

/// Example showing how to create a custom middleware
use std::fmt::Debug;
use async_trait::async_trait;
use crate::protocol::pipeline::Middleware;

#[derive(Debug)]
pub struct CustomLoggingMiddleware {
    prefix: String,
}

impl CustomLoggingMiddleware {
    pub fn new(prefix: String) -> Self {
        Self { prefix }
    }
}

#[async_trait]
impl Middleware for CustomLoggingMiddleware {
    async fn process(&self, context: MessageContext) -> Result<MessageContext> {
        tracing::info!("[{}] Processing message from client: {:?}", 
                      self.prefix, context.client_id);
        Ok(context)
    }
}

/// Example of creating a pipeline with custom middleware
pub async fn example_custom_middleware() -> Result<()> {
    let pipeline = PipelineBuilder::new()
        .add_middleware(CustomLoggingMiddleware::new("CUSTOM".to_string()))
        .with_parsing()
        .build();

    let context = MessageContext::new(
        r#"{"id":2,"method":"mining.submit","params":["user.worker","job123","nonce","result"]}"#.to_string()
    );

    let _result = pipeline.process(context).await?;
    Ok(())
}