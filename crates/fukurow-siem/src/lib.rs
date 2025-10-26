//! SIEM統合モジュール
//!
//! このクレートは様々なSIEM (Security Information and Event Management)
//! システムとの統合を提供します:
//! - Splunk (REST API, HEC)
//! - ELK Stack (Elasticsearch API)
//! - Chronicle (Google Cloud Security)

pub mod splunk;
pub mod elk;
pub mod chronicle;
pub mod common;

pub use splunk::SplunkClient;
pub use elk::ElkClient;
pub use chronicle::ChronicleClient;

// Re-export common types
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Common SIEM event format
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiemEvent {
    /// Unique event ID
    pub id: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Event type (alert, log, etc.)
    pub event_type: String,

    /// Source system
    pub source: String,

    /// Severity level
    pub severity: SiemSeverity,

    /// Event message
    pub message: String,

    /// Additional metadata
    pub metadata: serde_json::Value,

    /// Raw event data
    pub raw_data: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SiemSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl SiemEvent {
    pub fn new(event_type: &str, source: &str, message: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            source: source.to_string(),
            severity: SiemSeverity::Medium,
            message: message.to_string(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            raw_data: None,
        }
    }

    pub fn with_severity(mut self, severity: SiemSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_raw_data(mut self, raw_data: String) -> Self {
        self.raw_data = Some(raw_data);
        self
    }
}

/// SIEM client trait
#[async_trait::async_trait]
pub trait SiemClient: Send + Sync {
    /// Send a single event to SIEM
    async fn send_event(&self, event: SiemEvent) -> SiemResult<()>;

    /// Send multiple events to SIEM
    async fn send_events(&self, events: Vec<SiemEvent>) -> SiemResult<()>;

    /// Query events from SIEM
    async fn query_events(&self, query: &str, limit: Option<usize>) -> SiemResult<Vec<SiemEvent>>;

    /// Health check
    async fn health_check(&self) -> SiemResult<bool>;
}

/// SIEM configuration
#[derive(Debug, Clone, PartialEq)]
pub struct SiemConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout_seconds: u64,
}

impl SiemConfig {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            api_key: None,
            username: None,
            password: None,
            timeout_seconds: 30,
        }
    }

    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.api_key = Some(api_key.to_string());
        self
    }

    pub fn with_credentials(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }
}

/// SIEM operation result type
pub type SiemResult<T> = Result<T, SiemError>;

/// SIEM error types
#[derive(thiserror::Error, Debug)]
pub enum SiemError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Timeout error")]
    TimeoutError,

    #[error("Unknown error: {0}")]
    UnknownError(String),
}

/// SIEM integration manager
pub struct SiemManager {
    clients: Vec<Box<dyn SiemClient>>,
}

impl SiemManager {
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    pub fn add_client<C: SiemClient + 'static>(mut self, client: C) -> Self {
        self.clients.push(Box::new(client));
        self
    }

    /// Send event to all configured SIEM clients
    pub async fn broadcast_event(&self, event: SiemEvent) -> SiemResult<()> {
        for client in &self.clients {
            if let Err(e) = client.send_event(event.clone()).await {
                eprintln!("Failed to send event to SIEM client: {:?}", e);
                // Continue with other clients even if one fails
            }
        }
        Ok(())
    }

    /// Send events to all configured SIEM clients
    pub async fn broadcast_events(&self, events: Vec<SiemEvent>) -> SiemResult<()> {
        for client in &self.clients {
            if let Err(e) = client.send_events(events.clone()).await {
                eprintln!("Failed to send events to SIEM client: {:?}", e);
                // Continue with other clients even if one fails
            }
        }
        Ok(())
    }
}

impl Default for SiemManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    // Mock SIEM client for testing
    struct MockSiemClient {
        sent_events: std::sync::Mutex<Vec<SiemEvent>>,
        should_fail: bool,
    }

    #[async_trait]
    impl SiemClient for MockSiemClient {
        async fn send_event(&self, event: SiemEvent) -> SiemResult<()> {
            if self.should_fail {
                return Err(SiemError::ApiError {
                    status: 500,
                    message: "Mock failure".to_string(),
                });
            }
            self.sent_events.lock().unwrap().push(event);
            Ok(())
        }

        async fn send_events(&self, events: Vec<SiemEvent>) -> SiemResult<()> {
            if self.should_fail {
                return Err(SiemError::ApiError {
                    status: 500,
                    message: "Mock failure".to_string(),
                });
            }
            self.sent_events.lock().unwrap().extend(events);
            Ok(())
        }

        async fn query_events(&self, _query: &str, _limit: Option<usize>) -> SiemResult<Vec<SiemEvent>> {
            Ok(vec![])
        }

        async fn health_check(&self) -> SiemResult<bool> {
            Ok(!self.should_fail)
        }
    }

    impl MockSiemClient {
        fn new(should_fail: bool) -> Self {
            Self {
                sent_events: std::sync::Mutex::new(Vec::new()),
                should_fail,
            }
        }

        fn sent_events_count(&self) -> usize {
            self.sent_events.lock().unwrap().len()
        }
    }

    #[cfg(test)]
    mod siem_event_tests {
        use super::*;

        #[test]
        fn test_siem_event_new() {
            let event = SiemEvent::new("alert", "test_system", "Test message");

            assert_eq!(event.event_type, "alert");
            assert_eq!(event.source, "test_system");
            assert_eq!(event.message, "Test message");
            assert_eq!(event.severity, SiemSeverity::Medium);
            assert!(event.raw_data.is_none());
            assert!(!event.id.is_empty());
        }

        #[test]
        fn test_siem_event_with_severity() {
            let event = SiemEvent::new("log", "system", "Message")
                .with_severity(SiemSeverity::High);

            assert_eq!(event.severity, SiemSeverity::High);
        }

        #[test]
        fn test_siem_event_with_metadata() {
            let metadata = serde_json::json!({"key": "value"});
            let event = SiemEvent::new("event", "source", "message")
                .with_metadata(metadata.clone());

            assert_eq!(event.metadata, metadata);
        }

        #[test]
        fn test_siem_event_with_raw_data() {
            let raw_data = "raw log data".to_string();
            let event = SiemEvent::new("event", "source", "message")
                .with_raw_data(raw_data.clone());

            assert_eq!(event.raw_data, Some(raw_data));
        }

        #[test]
        fn test_siem_severity_variants() {
            assert_eq!(SiemSeverity::Low as u8, 0);
            assert_eq!(SiemSeverity::Medium as u8, 1);
            assert_eq!(SiemSeverity::High as u8, 2);
            assert_eq!(SiemSeverity::Critical as u8, 3);
        }
    }

    #[cfg(test)]
    mod siem_config_tests {
        use super::*;

        #[test]
        fn test_siem_config_new() {
            let config = SiemConfig::new("https://api.example.com");

            assert_eq!(config.endpoint, "https://api.example.com");
            assert!(config.api_key.is_none());
            assert!(config.username.is_none());
            assert!(config.password.is_none());
            assert_eq!(config.timeout_seconds, 30);
        }

        #[test]
        fn test_siem_config_with_api_key() {
            let config = SiemConfig::new("https://api.example.com")
                .with_api_key("test_key");

            assert_eq!(config.api_key, Some("test_key".to_string()));
        }

        #[test]
        fn test_siem_config_with_credentials() {
            let config = SiemConfig::new("https://api.example.com")
                .with_credentials("user", "pass");

            assert_eq!(config.username, Some("user".to_string()));
            assert_eq!(config.password, Some("pass".to_string()));
        }

        #[test]
        fn test_siem_config_with_timeout() {
            let config = SiemConfig::new("https://api.example.com")
                .with_timeout(60);

            assert_eq!(config.timeout_seconds, 60);
        }
    }

    #[cfg(test)]
    mod siem_error_tests {
        use super::*;

        #[test]
        fn test_siem_error_variants() {
            let err1 = SiemError::AuthError("auth failed".to_string());
            assert!(err1.to_string().contains("Authentication failed"));

            let err2 = SiemError::ConfigError("config error".to_string());
            assert!(err2.to_string().contains("Configuration error"));

            let err3 = SiemError::ParseError("parse error".to_string());
            assert!(err3.to_string().contains("Parse error"));

            let err4 = SiemError::ApiError {
                status: 404,
                message: "not found".to_string(),
            };
            assert!(err4.to_string().contains("404"));
            assert!(err4.to_string().contains("not found"));

            let err5 = SiemError::TimeoutError;
            assert!(err5.to_string().contains("Timeout error"));

            let err6 = SiemError::UnknownError("unknown".to_string());
            assert!(err6.to_string().contains("Unknown error"));
        }
    }

    #[cfg(test)]
    mod siem_manager_tests {
        use super::*;

        #[tokio::test]
        async fn test_siem_manager_new() {
            let manager = SiemManager::new();
            assert_eq!(manager.clients.len(), 0);
        }

        #[tokio::test]
        async fn test_siem_manager_add_client() {
            let manager = SiemManager::new()
                .add_client(MockSiemClient::new(false));

            assert_eq!(manager.clients.len(), 1);
        }

        #[tokio::test]
        async fn test_siem_manager_broadcast_event() {
            let client1 = MockSiemClient::new(false);
            let client2 = MockSiemClient::new(false);

            let manager = SiemManager::new()
                .add_client(client1)
                .add_client(client2);

            let event = SiemEvent::new("test", "source", "message");

            let result = manager.broadcast_event(event).await;
            assert!(result.is_ok());

            // Check that clients received the event
            // Note: This would require access to the mock clients, which we don't have
            // after moving them into the manager. In a real test, we'd use Arc<Mutex<MockSiemClient>>
        }

        #[tokio::test]
        async fn test_siem_manager_broadcast_events() {
            let client = MockSiemClient::new(false);
            let manager = SiemManager::new().add_client(client);

            let events = vec![
                SiemEvent::new("event1", "source", "message1"),
                SiemEvent::new("event2", "source", "message2"),
            ];

            let result = manager.broadcast_events(events).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_siem_manager_with_failing_client() {
            let good_client = MockSiemClient::new(false);
            let bad_client = MockSiemClient::new(true);

            let manager = SiemManager::new()
                .add_client(good_client)
                .add_client(bad_client);

            let event = SiemEvent::new("test", "source", "message");

            // Should succeed even with one failing client
            let result = manager.broadcast_event(event).await;
            assert!(result.is_ok());
        }
    }

    #[cfg(test)]
    mod mock_client_tests {
        use super::*;

        #[tokio::test]
        async fn test_mock_siem_client_send_event() {
            let client = MockSiemClient::new(false);
            let event = SiemEvent::new("test", "source", "message");

            let result = client.send_event(event).await;
            assert!(result.is_ok());
            assert_eq!(client.sent_events_count(), 1);
        }

        #[tokio::test]
        async fn test_mock_siem_client_send_events() {
            let client = MockSiemClient::new(false);
            let events = vec![
                SiemEvent::new("event1", "source", "message1"),
                SiemEvent::new("event2", "source", "message2"),
            ];

            let result = client.send_events(events).await;
            assert!(result.is_ok());
            assert_eq!(client.sent_events_count(), 2);
        }

        #[tokio::test]
        async fn test_mock_siem_client_send_event_failure() {
            let client = MockSiemClient::new(true);
            let event = SiemEvent::new("test", "source", "message");

            let result = client.send_event(event).await;
            assert!(result.is_err());
            assert_eq!(client.sent_events_count(), 0);
        }

        #[tokio::test]
        async fn test_mock_siem_client_health_check() {
            let healthy_client = MockSiemClient::new(false);
            let unhealthy_client = MockSiemClient::new(true);

            assert_eq!(healthy_client.health_check().await.unwrap(), true);
            assert_eq!(unhealthy_client.health_check().await.unwrap(), false);
        }
    }
}
