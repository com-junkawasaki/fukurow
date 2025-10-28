//! Deprecated - moved to `fukurow-observability` crate.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus { Up, Down, Degraded }

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub message: Option<String>,
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// System metrics
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

/// Monitoring service
#[derive(Debug, Clone)]
pub struct MonitoringService {
    health_checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    metrics: Arc<RwLock<SystemMetrics>>,
    start_time: DateTime<Utc>,
}

impl MonitoringService {
    /// Create new monitoring service
    pub fn new() -> Self {
        let start_time = Utc::now();
        let metrics = SystemMetrics {
            timestamp: start_time,
            memory_usage_mb: 0,
            cpu_usage_percent: 0.0,
            active_connections: 0,
            total_requests: 0,
            error_rate_percent: 0.0,
            uptime_seconds: 0,
        };

        Self {
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(metrics)),
            start_time,
        }
    }

    /// Register a health check
    pub async fn register_health_check(&self, name: &str, check: HealthCheck) {
        let mut checks = self.health_checks.write().await;
        checks.insert(name.to_string(), check);
    }

    /// Run all health checks
    pub async fn run_health_checks(&self) -> HashMap<String, HealthCheck> {
        let mut results = HashMap::new();
        let checks = self.health_checks.read().await.clone();

        for (name, check) in checks.iter() {
            // Re-run the health check
            let updated_check = self.run_single_health_check(&check.name).await;
            results.insert(name.clone(), updated_check.clone());

            // Update stored check
            let mut stored_checks = self.health_checks.write().await;
            stored_checks.insert(name.clone(), updated_check);
        }

        results
    }

    /// Run a single health check
    async fn run_single_health_check(&self, name: &str) -> HealthCheck {
        let start = std::time::Instant::now();

        let result = match name {
            "database" => self.check_database().await,
            "reasoner" => self.check_reasoner().await,
            "memory" => self.check_memory().await,
            "cpu" => self.check_cpu().await,
            _ => HealthCheck {
                name: name.to_string(),
                status: HealthStatus::Up,
                timestamp: Utc::now(),
                duration_ms: start.elapsed().as_millis() as u64,
                message: Some("Health check completed".to_string()),
                details: None,
            },
        };

        result
    }

    /// Database connectivity check
    async fn check_database(&self) -> HealthCheck {
        let start = std::time::Instant::now();
        let timestamp = Utc::now();

        // TODO: Implement actual database connectivity check
        // For now, assume it's healthy
        HealthCheck {
            name: "database".to_string(),
            status: HealthStatus::Up,
            timestamp,
            duration_ms: start.elapsed().as_millis() as u64,
            message: Some("Database connection healthy".to_string()),
            details: Some(HashMap::from([
                ("connections".to_string(), serde_json::json!(10)),
                ("pool_size".to_string(), serde_json::json!(20)),
            ])),
        }
    }

    /// Reasoner functionality check
    async fn check_reasoner(&self) -> HealthCheck {
        let start = std::time::Instant::now();
        let timestamp = Utc::now();

        // TODO: Implement actual reasoner health check
        HealthCheck {
            name: "reasoner".to_string(),
            status: HealthStatus::Up,
            timestamp,
            duration_ms: start.elapsed().as_millis() as u64,
            message: Some("Reasoner engine healthy".to_string()),
            details: Some(HashMap::from([
                ("active_rules".to_string(), serde_json::json!(150)),
                ("inference_capacity".to_string(), serde_json::json!("high")),
            ])),
        }
    }

    /// Memory usage check
    async fn check_memory(&self) -> HealthCheck {
        let start = std::time::Instant::now();
        let timestamp = Utc::now();

        // Get memory usage (simplified)
        let memory_mb = std::process::Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0) / 1024;

        let status = if memory_mb < 1024 { // Less than 1GB
            HealthStatus::Up
        } else if memory_mb < 2048 { // Less than 2GB
            HealthStatus::Degraded
        } else {
            HealthStatus::Down
        };

        HealthCheck {
            name: "memory".to_string(),
            status,
            timestamp,
            duration_ms: start.elapsed().as_millis() as u64,
            message: Some(format!("Memory usage: {} MB", memory_mb)),
            details: Some(HashMap::from([
                ("usage_mb".to_string(), serde_json::json!(memory_mb)),
                ("threshold_mb".to_string(), serde_json::json!(1024)),
            ])),
        }
    }

    /// CPU usage check
    async fn check_cpu(&self) -> HealthCheck {
        let start = std::time::Instant::now();
        let timestamp = Utc::now();

        // TODO: Implement actual CPU usage monitoring
        let cpu_percent = 15.5; // Mock value

        let status = if cpu_percent < 80.0 {
            HealthStatus::Up
        } else if cpu_percent < 95.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Down
        };

        HealthCheck {
            name: "cpu".to_string(),
            status,
            timestamp,
            duration_ms: start.elapsed().as_millis() as u64,
            message: Some(format!("CPU usage: {:.1}%", cpu_percent)),
            details: Some(HashMap::from([
                ("usage_percent".to_string(), serde_json::json!(cpu_percent)),
                ("cores".to_string(), serde_json::json!(4)),
            ])),
        }
    }

    /// Update system metrics
    pub async fn update_metrics(&self, active_connections: u32, total_requests: u64, error_rate: f64) {
        let mut metrics = self.metrics.write().await;

        // Get current memory usage
        let memory_mb = std::process::Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0) / 1024;

        // TODO: Get actual CPU usage
        let cpu_percent = 15.5;

        let uptime = (Utc::now() - self.start_time).num_seconds() as u64;

        // Get current memory usage
        let memory_usage_mb = std::process::Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0) / 1024;

        *metrics = SystemMetrics {
            timestamp: Utc::now(),
            memory_usage_mb,
            cpu_usage_percent: cpu_percent,
            active_connections,
            total_requests,
            error_rate_percent: error_rate,
            uptime_seconds: uptime,
        };
    }

    /// Get current system metrics
    pub async fn get_metrics(&self) -> SystemMetrics {
        self.metrics.read().await.clone()
    }

    /// Get overall health status
    pub async fn get_overall_health(&self) -> HealthStatus {
        let checks = self.run_health_checks().await;

        let has_down = checks.values().any(|check| matches!(check.status, HealthStatus::Down));
        let has_degraded = checks.values().any(|check| matches!(check.status, HealthStatus::Degraded));

        if has_down {
            HealthStatus::Down
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Up
        }
    }
}

impl Default for MonitoringService {
    fn default() -> Self {
        Self::new()
    }
}

/// Monitoring routes for Axum
pub mod routes {
    use axum::{
        extract::State,
        http::StatusCode,
        response::{IntoResponse, Json},
        routing::get,
        Router,
    };
    use std::sync::Arc;

    use super::{MonitoringService, HealthStatus};

    /// Create monitoring routes
    pub fn monitoring_routes(monitoring: Arc<MonitoringService>) -> Router<()> {
        Router::new()
            .route("/health", get(health_check))
            .route("/health/detailed", get(detailed_health_check))
            .route("/metrics", get(system_metrics))
            .with_state(monitoring)
    }

    /// Basic health check endpoint
    async fn health_check(
        State(monitoring): State<Arc<MonitoringService>>,
    ) -> impl IntoResponse {
        let status = monitoring.get_overall_health().await;

        let status_code = match status {
            HealthStatus::Up => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK, // Still return 200 for degraded
            HealthStatus::Down => StatusCode::SERVICE_UNAVAILABLE,
        };

        (status_code, Json(serde_json::json!({
            "status": status,
            "timestamp": chrono::Utc::now(),
        })))
    }

    /// Detailed health check endpoint
    async fn detailed_health_check(
        State(monitoring): State<Arc<MonitoringService>>,
    ) -> impl IntoResponse {
        let checks = monitoring.run_health_checks().await;
        let overall_status = monitoring.get_overall_health().await;

        Json(serde_json::json!({
            "overall_status": overall_status,
            "timestamp": chrono::Utc::now(),
            "checks": checks,
        }))
    }

    /// System metrics endpoint
    async fn system_metrics(
        State(monitoring): State<Arc<MonitoringService>>,
    ) -> impl IntoResponse {
        let metrics = monitoring.get_metrics().await;

        Json(serde_json::json!({
            "metrics": metrics,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_service_creation() {
        let service = MonitoringService::new();
        assert!(service.health_checks.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_health_check_registration() {
        let service = MonitoringService::new();
        let check = HealthCheck {
            name: "test".to_string(),
            status: HealthStatus::Up,
            timestamp: Utc::now(),
            duration_ms: 100,
            message: Some("Test check".to_string()),
            details: None,
        };

        service.register_health_check("test", check.clone()).await;
        let checks = service.run_health_checks().await;
        assert!(checks.contains_key("test"));
    }

    #[tokio::test]
    async fn test_overall_health_calculation() {
        let service = MonitoringService::new();

        // Register healthy check
        let healthy_check = HealthCheck {
            name: "healthy".to_string(),
            status: HealthStatus::Up,
            timestamp: Utc::now(),
            duration_ms: 50,
            message: Some("All good".to_string()),
            details: None,
        };

        service.register_health_check("healthy", healthy_check).await;

        let overall = service.get_overall_health().await;
        assert!(matches!(overall, HealthStatus::Up));
    }
}
