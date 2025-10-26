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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone)]
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
