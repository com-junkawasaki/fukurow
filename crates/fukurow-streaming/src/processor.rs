//! # Stream Processor
//!
//! Core streaming processor for handling events

use crate::{StreamingEvent, StreamingConfig};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Stream processor trait
#[async_trait]
pub trait StreamProcessor: Send + Sync {
    /// Process a single event
    async fn process_event(&self, event: StreamingEvent) -> Result<(), StreamError>;

    /// Process a batch of events
    async fn process_batch(&self, events: Vec<StreamingEvent>) -> Result<(), StreamError>;

    /// Get processor name
    fn name(&self) -> &'static str;

    /// Get processor health status
    async fn health_check(&self) -> Result<(), StreamError>;
}

/// Event stream processor
pub struct EventStreamProcessor<P: StreamProcessor> {
    processor: Arc<P>,
    config: StreamingConfig,
    event_tx: mpsc::UnboundedSender<StreamingEvent>,
    event_rx: mpsc::UnboundedReceiver<StreamingEvent>,
}

impl<P: StreamProcessor> EventStreamProcessor<P> {
    /// Create a new event stream processor
    pub fn new(processor: P, config: StreamingConfig) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Self {
            processor: Arc::new(processor),
            config,
            event_tx,
            event_rx,
        }
    }

    /// Start processing events
    pub async fn start_processing(mut self) -> Result<(), StreamError> {
        info!("Starting event stream processor: {}", self.processor.name());

        let processor = Arc::clone(&self.processor);
        let batch_size = self.config.processing.batch_size;
        let processing_timeout = std::time::Duration::from_secs(
            self.config.processing.processing_timeout_seconds
        );

        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(batch_size);
            let mut last_process_time = std::time::Instant::now();

            while let Some(event) = self.event_rx.recv().await {
                batch.push(event);

                // Process batch if it's full or timeout has passed
                let should_process = batch.len() >= batch_size ||
                    last_process_time.elapsed() >= processing_timeout;

                if should_process {
                    if let Err(e) = processor.process_batch(batch).await {
                        error!("Failed to process batch: {}", e);
                    }
                    batch = Vec::with_capacity(batch_size);
                    last_process_time = std::time::Instant::now();
                }
            }

            // Process remaining events
            if !batch.is_empty() {
                if let Err(e) = processor.process_batch(batch).await {
                    error!("Failed to process final batch: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Send event to processor
    pub fn send_event(&self, event: StreamingEvent) -> Result<(), StreamError> {
        self.event_tx.send(event)
            .map_err(|_| StreamError::ChannelClosed)
    }

    /// Create event sender handle
    pub fn event_sender(&self) -> EventSender {
        EventSender {
            sender: self.event_tx.clone(),
        }
    }

    /// Get processor health
    pub async fn health_check(&self) -> Result<(), StreamError> {
        self.processor.health_check().await
    }
}

/// Event sender handle for external components
#[derive(Clone)]
pub struct EventSender {
    sender: mpsc::UnboundedSender<StreamingEvent>,
}

impl EventSender {
    /// Send an event
    pub fn send(&self, event: StreamingEvent) -> Result<(), StreamError> {
        self.sender.send(event)
            .map_err(|_| StreamError::ChannelClosed)
    }

    /// Send security event
    pub fn send_security_event(&self, event: fukurow_core::model::CyberEvent, source: String) -> Result<(), StreamError> {
        let streaming_event = StreamingEvent::SecurityEvent {
            event,
            timestamp: chrono::Utc::now(),
            source,
        };
        self.send(streaming_event)
    }

    /// Send reasoning result
    pub fn send_reasoning_result(&self, actions: Vec<fukurow_core::model::SecurityAction>, execution_time_ms: u64, event_count: usize) -> Result<(), StreamError> {
        let streaming_event = StreamingEvent::ReasoningResult {
            actions,
            execution_time_ms,
            event_count,
            timestamp: chrono::Utc::now(),
        };
        self.send(streaming_event)
    }

    /// Send anomaly detection result
    pub fn send_anomaly(&self, score: f64, threshold: f64, metric: String) -> Result<(), StreamError> {
        let streaming_event = StreamingEvent::AnomalyDetected {
            score,
            threshold,
            metric,
            timestamp: chrono::Utc::now(),
        };
        self.send(streaming_event)
    }

    /// Send system metrics
    pub fn send_metrics(&self, cpu_usage: f64, memory_usage: f64, active_connections: u32) -> Result<(), StreamError> {
        let streaming_event = StreamingEvent::SystemMetrics {
            cpu_usage,
            memory_usage,
            active_connections,
            timestamp: chrono::Utc::now(),
        };
        self.send(streaming_event)
    }
}

/// Stream consumer trait
#[async_trait]
pub trait StreamConsumer: Send + Sync {
    /// Consume events from stream
    async fn consume(&self) -> Pin<Box<dyn Stream<Item = Result<StreamingEvent, StreamError>> + Send>>;

    /// Get consumer name
    fn name(&self) -> &'static str;

    /// Get consumer health status
    async fn health_check(&self) -> Result<(), StreamError>;
}

/// Stream producer trait
#[async_trait]
pub trait StreamProducer: Send + Sync {
    /// Produce event to stream
    async fn produce(&self, event: StreamingEvent) -> Result<(), StreamError>;

    /// Produce batch of events
    async fn produce_batch(&self, events: Vec<StreamingEvent>) -> Result<(), StreamError>;

    /// Get producer name
    fn name(&self) -> &'static str;

    /// Get producer health status
    async fn health_check(&self) -> Result<(), StreamError>;
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

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Processor error: {0}")]
    ProcessorError(String),

    #[error("Health check failed: {0}")]
    HealthCheckError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_core::model::CyberEvent;

    struct MockProcessor;

    #[async_trait]
    impl StreamProcessor for MockProcessor {
        async fn process_event(&self, _event: StreamingEvent) -> Result<(), StreamError> {
            Ok(())
        }

        async fn process_batch(&self, events: Vec<StreamingEvent>) -> Result<(), StreamError> {
            info!("Processed batch of {} events", events.len());
            Ok(())
        }

        fn name(&self) -> &'static str {
            "mock_processor"
        }

        async fn health_check(&self) -> Result<(), StreamError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_sender() {
        let processor = MockProcessor;
        let config = StreamingConfig::default();
        let stream_processor = EventStreamProcessor::new(processor, config);

        let sender = stream_processor.event_sender();

        // Send security event
        let cyber_event = CyberEvent::NetworkConnection {
            source_ip: "192.168.1.1".to_string(),
            dest_ip: "10.0.0.1".to_string(),
            port: 443,
            protocol: "tcp".to_string(),
            timestamp: 1640995200,
        };

        sender.send_security_event(cyber_event, "test_sensor".to_string()).unwrap();

        // Send reasoning result
        let actions = vec![];
        sender.send_reasoning_result(actions, 150, 5).unwrap();

        // Send anomaly
        sender.send_anomaly(2.5, 2.0, "login_attempts".to_string()).unwrap();

        // Send metrics
        sender.send_metrics(45.5, 67.8, 150).unwrap();

        // Health check
        assert!(stream_processor.health_check().await.is_ok());
    }

    #[test]
    fn test_stream_error_display() {
        let err = StreamError::ChannelClosed;
        assert_eq!(err.to_string(), "Channel closed");

        let err = StreamError::ConnectionError("connection failed".to_string());
        assert_eq!(err.to_string(), "Connection error: connection failed");
    }
}
