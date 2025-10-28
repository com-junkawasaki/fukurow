//! # Streaming Configuration
//!
//! Configuration for streaming processors and connections

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Streaming processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Processor type
    pub processor_type: ProcessorType,

    /// Connection configuration
    pub connection: ConnectionConfig,

    /// Processing configuration
    pub processing: ProcessingConfig,

    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

/// Processor type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessorType {
    Kafka,
    NATS,
    Redis,
    RabbitMQ,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "config")]
pub enum ConnectionConfig {
    /// Kafka connection
    Kafka(KafkaConfig),

    /// NATS connection
    NATS(NATSConfig),

    /// Redis connection
    Redis(RedisConfig),

    /// RabbitMQ connection
    RabbitMQ(RabbitMQConfig),
}

/// Kafka configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Bootstrap servers
    pub bootstrap_servers: Vec<String>,

    /// Group ID
    pub group_id: String,

    /// Topics to consume from
    pub consume_topics: Vec<String>,

    /// Topic to produce to
    pub produce_topic: String,

    /// Additional Kafka properties
    pub properties: HashMap<String, String>,
}

/// NATS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NATSConfig {
    /// NATS server URLs
    pub servers: Vec<String>,

    /// Subject to subscribe to
    pub subject: String,

    /// Queue group for load balancing
    pub queue_group: Option<String>,

    /// Authentication credentials
    pub credentials: Option<String>,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis server URL
    pub url: String,

    /// Stream key to consume from
    pub stream_key: String,

    /// Consumer group name
    pub consumer_group: String,

    /// Consumer name
    pub consumer_name: String,

    /// Database number
    pub database: Option<u8>,
}

/// RabbitMQ configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RabbitMQConfig {
    /// RabbitMQ server URL
    pub url: String,

    /// Exchange name
    pub exchange: String,

    /// Routing key
    pub routing_key: String,

    /// Queue name
    pub queue: String,

    /// Exchange type
    pub exchange_type: String,
}

/// Processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Batch size for processing
    pub batch_size: usize,

    /// Processing timeout in seconds
    pub processing_timeout_seconds: u64,

    /// Maximum concurrent processors
    pub max_concurrent_processors: usize,

    /// Buffer size for internal queues
    pub buffer_size: usize,

    /// Retry configuration
    pub retry: RetryConfig,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64,

    /// Maximum backoff duration in milliseconds
    pub max_backoff_ms: u64,

    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Metrics prefix
    pub metrics_prefix: String,

    /// Enable health checks
    pub enable_health_checks: bool,

    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            processor_type: ProcessorType::Kafka,
            connection: ConnectionConfig::Kafka(KafkaConfig {
                bootstrap_servers: vec!["localhost:9092".to_string()],
                group_id: "fukurow-streaming".to_string(),
                consume_topics: vec!["security-events".to_string()],
                produce_topic: "reasoning-results".to_string(),
                properties: HashMap::new(),
            }),
            processing: ProcessingConfig {
                batch_size: 100,
                processing_timeout_seconds: 30,
                max_concurrent_processors: 10,
                buffer_size: 1000,
                retry: RetryConfig {
                    max_attempts: 3,
                    initial_backoff_ms: 100,
                    max_backoff_ms: 10000,
                    backoff_multiplier: 2.0,
                },
            },
            monitoring: MonitoringConfig {
                enable_metrics: true,
                metrics_prefix: "fukurow_streaming".to_string(),
                enable_health_checks: true,
                health_check_interval_seconds: 30,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = StreamingConfig::default();

        match &config.connection {
            ConnectionConfig::Kafka(kafka_config) => {
                assert_eq!(kafka_config.bootstrap_servers, vec!["localhost:9092".to_string()]);
                assert_eq!(kafka_config.group_id, "fukurow-streaming");
            }
            _ => panic!("Expected Kafka config"),
        }

        assert_eq!(config.processing.batch_size, 100);
        assert_eq!(config.monitoring.metrics_prefix, "fukurow_streaming");
    }

    #[test]
    fn test_kafka_config_serialization() {
        let kafka_config = KafkaConfig {
            bootstrap_servers: vec!["kafka1:9092".to_string(), "kafka2:9092".to_string()],
            group_id: "test-group".to_string(),
            consume_topics: vec!["events".to_string()],
            produce_topic: "results".to_string(),
            properties: HashMap::from([
                ("auto.offset.reset".to_string(), "earliest".to_string()),
            ]),
        };

        let json = serde_json::to_string(&kafka_config).unwrap();
        let deserialized: KafkaConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.bootstrap_servers.len(), 2);
        assert_eq!(deserialized.group_id, "test-group");
    }

    #[test]
    fn test_retry_config() {
        let retry = RetryConfig {
            max_attempts: 5,
            initial_backoff_ms: 200,
            max_backoff_ms: 30000,
            backoff_multiplier: 1.5,
        };

        assert_eq!(retry.max_attempts, 5);
        assert_eq!(retry.backoff_multiplier, 1.5);
    }
}
