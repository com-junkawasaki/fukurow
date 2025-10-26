//! API data models

use reasoner_graph::model::{CyberEvent, SecurityAction};
use serde::{Deserialize, Serialize};

/// API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: i64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Event submission request
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitEventRequest {
    pub event: CyberEvent,
}

/// Reasoning request
#[derive(Debug, Deserialize)]
pub struct ReasoningRequest {
    pub include_details: Option<bool>,
}

/// Reasoning response
#[derive(Debug, Serialize)]
pub struct ReasoningResponse {
    pub actions: Vec<SecurityAction>,
    pub execution_time_ms: u64,
    pub event_count: usize,
}

/// Graph query request
#[derive(Debug, Deserialize)]
pub struct GraphQueryRequest {
    pub subject: Option<String>,
    pub predicate: Option<String>,
    pub object: Option<String>,
    pub graph_name: Option<String>,
}

/// Graph query response
#[derive(Debug, Serialize)]
pub struct GraphQueryResponse {
    pub triples: Vec<reasoner_graph::model::Triple>,
    pub count: usize,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Statistics response
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_events: usize,
    pub total_actions: usize,
    pub uptime_seconds: u64,
    pub memory_usage_mb: Option<f64>,
}

/// Rule management request
#[derive(Debug, Deserialize)]
pub struct AddRuleRequest {
    pub rule: reasoner_graph::model::InferenceRule,
}

/// Rules list response
#[derive(Debug, Serialize)]
pub struct RulesResponse {
    pub rules: Vec<reasoner_graph::model::InferenceRule>,
    pub count: usize,
}

/// Threat intelligence response
#[derive(Debug, Serialize)]
pub struct ThreatIntelResponse {
    pub indicators_count: usize,
    pub sources_count: usize,
    pub last_updated: i64,
    pub statistics: std::collections::HashMap<String, usize>,
}

/// Error types for API
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Event processing error: {0}")]
    EventProcessingError(String),

    #[error("Reasoning error: {0}")]
    ReasoningError(String),

    #[error("Graph operation error: {0}")]
    GraphError(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl From<reasoner_core::ReasonerError> for ApiError {
    fn from(err: reasoner_core::ReasonerError) -> Self {
        match err {
            reasoner_core::ReasonerError::GraphError(_) => ApiError::GraphError(err.to_string()),
            reasoner_core::ReasonerError::RuleError(_) => ApiError::ReasoningError(err.to_string()),
            reasoner_core::ReasonerError::ContextError(_) => ApiError::InternalError(err.to_string()),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}
