//! # Stream Abstraction
//!
//! Abstract stream interface for different streaming platforms

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::stream::Stream;

/// Stream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub stream_type: StreamType,
    pub topic: String,
    pub group_id: Option<String>,
    pub partition: Option<i32>,
    pub options: std::collections::HashMap<String, String>,
}

/// Stream type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamType {
    Kafka,
    NATS,
    Redis,
    RabbitMQ,
}

/// Abstract stream interface
#[async_trait]
pub trait AbstractStream: Send + Sync {
    /// Send a message to the stream
    async fn send(&self, key: Option<&str>, payload: &[u8]) -> Result<(), StreamError>;

    /// Receive messages from the stream
    async fn receive(&self) -> Result<Pin<Box<dyn Stream<Item = Result<StreamMessage, StreamError>> + Send>>, StreamError>;

    /// Close the stream connection
    async fn close(&self) -> Result<(), StreamError>;
}

/// Stream message
#[derive(Debug, Clone)]
pub struct StreamMessage {
    pub key: Option<String>,
    pub payload: Vec<u8>,
    pub timestamp: Option<i64>,
    pub headers: std::collections::HashMap<String, String>,
}

/// Stream error
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Channel closed")]
    ChannelClosed,

    #[error("Processing timeout")]
    ProcessingTimeout,

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Send error: {0}")]
    SendError(String),

    #[error("Receive error: {0}")]
    ReceiveError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Processor error: {0}")]
    ProcessorError(String),

    #[error("Health check failed: {0}")]
    HealthCheckError(String),

    #[error("Stream closed")]
    StreamClosed,
}
