//! # Fukurow Streaming
//!
//! Real-time streaming processing for Fukurow reasoning engine.
//! Supports Kafka, NATS, Redis Streams, and RabbitMQ.

pub mod stream;
pub mod processor;
pub mod consumer;
pub mod producer;
pub mod config;

pub use stream::*;
pub use processor::*;
pub use consumer::*;
pub use producer::*;
pub use config::*;

/// Streaming event types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StreamingEvent {
    /// Security event from sensors
    SecurityEvent {
        event: fukurow_core::model::CyberEvent,
        timestamp: chrono::DateTime<chrono::Utc>,
        source: String,
    },

    /// Reasoning result
    ReasoningResult {
        actions: Vec<fukurow_core::model::SecurityAction>,
        execution_time_ms: u64,
        event_count: usize,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Anomaly detection result
    AnomalyDetected {
        score: f64,
        threshold: f64,
        metric: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// System metrics
    SystemMetrics {
        cpu_usage: f64,
        memory_usage: f64,
        active_connections: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl StreamingEvent {
    /// Get event type as string
    pub fn event_type(&self) -> &'static str {
        match self {
            StreamingEvent::SecurityEvent { .. } => "security_event",
            StreamingEvent::ReasoningResult { .. } => "reasoning_result",
            StreamingEvent::AnomalyDetected { .. } => "anomaly_detected",
            StreamingEvent::SystemMetrics { .. } => "system_metrics",
        }
    }

    /// Get event timestamp
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            StreamingEvent::SecurityEvent { timestamp, .. } => *timestamp,
            StreamingEvent::ReasoningResult { timestamp, .. } => *timestamp,
            StreamingEvent::AnomalyDetected { timestamp, .. } => *timestamp,
            StreamingEvent::SystemMetrics { timestamp, .. } => *timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_core::model::CyberEvent;

    #[test]
    fn test_streaming_event_types() {
        let security_event = StreamingEvent::SecurityEvent {
            event: CyberEvent::NetworkConnection {
                source_ip: "192.168.1.1".to_string(),
                dest_ip: "10.0.0.1".to_string(),
                port: 443,
                protocol: "tcp".to_string(),
                timestamp: 1640995200,
            },
            timestamp: chrono::Utc::now(),
            source: "sensor1".to_string(),
        };

        assert_eq!(security_event.event_type(), "security_event");
        assert!(security_event.timestamp() <= chrono::Utc::now());
    }

    #[test]
    fn test_system_metrics_event() {
        let metrics_event = StreamingEvent::SystemMetrics {
            cpu_usage: 45.5,
            memory_usage: 67.8,
            active_connections: 150,
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(metrics_event.event_type(), "system_metrics");
        assert_eq!(metrics_event.timestamp() <= chrono::Utc::now(), true);
    }
}
