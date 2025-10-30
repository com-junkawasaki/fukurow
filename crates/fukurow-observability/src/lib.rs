//! Observability (health/metrics) abstractions and Axum routes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Up,
    Down,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub message: Option<String>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
    pub active_connections: u32,
    pub total_requests: u64,
    pub error_rate_percent: f64,
    pub uptime_seconds: u64,
}

#[async_trait::async_trait]
pub trait HealthMonitor: Send + Sync + 'static {
    async fn get_overall_health(&self) -> HealthStatus;
    async fn run_health_checks(&self) -> Vec<HealthCheck>;
    async fn get_metrics(&self) -> SystemMetrics;
}

/// Default health monitor implementation
#[derive(Clone)]
pub struct DefaultHealthMonitor {
    start_time: std::time::Instant,
    request_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl DefaultHealthMonitor {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            request_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn increment_request_count(&self) {
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[async_trait::async_trait]
impl HealthMonitor for DefaultHealthMonitor {
    async fn get_overall_health(&self) -> HealthStatus {
        HealthStatus::Up
    }

    async fn run_health_checks(&self) -> Vec<HealthCheck> {
        vec![
            HealthCheck {
                name: "api".to_string(),
                status: HealthStatus::Up,
                message: Some("API is responding".to_string()),
                timestamp: chrono::Utc::now(),
                duration_ms: 0,
                details: None,
            },
            HealthCheck {
                name: "reasoning".to_string(),
                status: HealthStatus::Up,
                message: Some("Reasoning engine is operational".to_string()),
                timestamp: chrono::Utc::now(),
                duration_ms: 0,
                details: None,
            },
        ]
    }

    async fn get_metrics(&self) -> SystemMetrics {
        SystemMetrics {
            timestamp: chrono::Utc::now(),
            memory_usage_mb: 0, // Placeholder
            cpu_usage_percent: 0.0, // Placeholder
            active_connections: 0, // Placeholder
            total_requests: self.request_count.load(std::sync::atomic::Ordering::Relaxed),
            error_rate_percent: 0.0,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }
}

pub mod metrics {
    /// Standard metric names for Fukurow components
    pub mod names {
        pub const REQUEST_TOTAL: &str = "fukurow_requests_total";
        pub const REQUEST_DURATION: &str = "fukurow_request_duration_seconds";
        pub const ACTIVE_CONNECTIONS: &str = "fukurow_active_connections";
        pub const TRIPLE_COUNT: &str = "fukurow_triple_count";
        pub const INFERENCE_TIME: &str = "fukurow_inference_duration_seconds";
        pub const REASONING_ERRORS: &str = "fukurow_reasoning_errors_total";
        pub const STREAM_EVENTS: &str = "fukurow_stream_events_total";
        pub const OPERATOR_RECONCILES: &str = "fukurow_operator_reconciles_total";
        pub const OPERATOR_RECONCILE_DURATION: &str = "fukurow_operator_reconcile_duration_seconds";
    }

    /// Standard labels for metrics
    pub mod labels {
        pub const SERVICE: &str = "service";
        pub const COMPONENT: &str = "component";
        pub const OPERATION: &str = "operation";
        pub const STATUS: &str = "status";
        pub const METHOD: &str = "method";
        pub const ENDPOINT: &str = "endpoint";
        pub const CLUSTER: &str = "cluster";
        pub const NAMESPACE: &str = "namespace";
        pub const EVENT_TYPE: &str = "event_type";
        pub const STREAM_TYPE: &str = "stream_type";
    }
}

pub mod tracing {
    /// Standard span names for Fukurow operations
    pub mod spans {
        pub const API_REQUEST: &str = "api.request";
        pub const REASONING_EXECUTE: &str = "reasoning.execute";
        pub const STORE_QUERY: &str = "store.query";
        pub const STREAM_SEND: &str = "stream.send";
        pub const OPERATOR_RECONCILE: &str = "operator.reconcile";
        pub const HEALTH_CHECK: &str = "health.check";
    }

    /// Standard span attributes
    pub mod attributes {
        pub const SERVICE_NAME: &str = "service.name";
        pub const OPERATION: &str = "operation";
        pub const COMPONENT: &str = "component";
        pub const USER_ID: &str = "user.id";
        pub const REQUEST_ID: &str = "request.id";
        pub const CLUSTER_ID: &str = "cluster.id";
        pub const NAMESPACE: &str = "namespace";
        pub const ENDPOINT: &str = "endpoint";
        pub const METHOD: &str = "method";
        pub const STATUS_CODE: &str = "status.code";
        pub const ERROR_TYPE: &str = "error.type";
        pub const DURATION_MS: &str = "duration.ms";
    }
}

pub mod routes {
    use super::*;
    use axum::{
        extract::State,
        http::StatusCode,
        response::{IntoResponse, Json},
        routing::get,
        Router,
    };
    use std::sync::Arc;

    pub fn monitoring_routes<H: HealthMonitor + 'static>(monitor: Arc<H>) -> Router {
        Router::new()
            .route("/health", get(health))
            .route("/health/detailed", get(health_detailed))
            .route("/metrics", get(metrics))
            .with_state(monitor)
    }

    async fn health<H: HealthMonitor>(State(m): State<Arc<H>>) -> impl IntoResponse {
        let status = m.get_overall_health().await;
        let status_code = match status {
            HealthStatus::Up => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK,
            HealthStatus::Down => StatusCode::SERVICE_UNAVAILABLE,
        };
        (status_code, Json(status))
    }

    async fn health_detailed<H: HealthMonitor>(State(m): State<Arc<H>>) -> impl IntoResponse {
        let checks = m.run_health_checks().await;
        Json(checks)
    }

    async fn metrics<H: HealthMonitor>(State(m): State<Arc<H>>) -> impl IntoResponse {
        let s = m.get_metrics().await;
        Json(s)
    }

}


