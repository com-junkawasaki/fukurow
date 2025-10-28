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

    pub fn monitoring_routes(monitor: Arc<dyn HealthMonitor>) -> Router {
        Router::new()
            .route("/health", get(health))
            .route("/health/detailed", get(health_detailed))
            .route("/metrics", get(metrics))
            .with_state(monitor)
    }

    async fn health(State(m): State<Arc<dyn HealthMonitor>>) -> impl IntoResponse {
        let status = m.get_overall_health().await;
        let status_code = match status {
            HealthStatus::Up => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK,
            HealthStatus::Down => StatusCode::SERVICE_UNAVAILABLE,
        };
        (status_code, Json(status))
    }

    async fn health_detailed(State(m): State<Arc<dyn HealthMonitor>>) -> impl IntoResponse {
        let checks = m.run_health_checks().await;
        Json(checks)
    }

    async fn metrics(State(m): State<Arc<dyn HealthMonitor>>) -> impl IntoResponse {
        let s = m.get_metrics().await;
        Json(s)
    }
}


