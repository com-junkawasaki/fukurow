//! API request handlers

use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::Json as JsonResponse,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Instant;

use crate::models::*;
use fukurow_observability::{HealthMonitor, HealthStatus, HealthCheck, SystemMetrics};
use fukurow_engine::ReasonerEngine;
use fukurow_domain_cyber::threat_intelligence::ThreatProcessor;

#[cfg(feature = "streaming")]
use fukurow_streaming::processor::EventSender;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub reasoner: Arc<ReasonerEngine>,
    pub threat_processor: Arc<RwLock<ThreatProcessor>>,
    pub monitoring: Arc<dyn HealthMonitor>,
    pub start_time: Instant,
    #[cfg(feature = "streaming")]
    pub event_sender: Option<EventSender>,
}

/// Health check handler
pub async fn health_check(Extension(state): Extension<Arc<AppState>>) -> JsonResponse<ApiResponse<HealthResponse>> {
    let uptime = state.start_time.elapsed();

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime.as_secs(),
    };

    JsonResponse(ApiResponse::success(response))
}

/// Submit cyber event handler
pub async fn submit_event(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<SubmitEventRequest>,
) -> Result<JsonResponse<ApiResponse<String>>, (StatusCode, JsonResponse<ApiResponse<String>>)> {
    match state.reasoner.add_event(request.event.clone()).await {
        Ok(_) => {
            // Send security event if streaming is enabled
            #[cfg(feature = "streaming")]
            if let Some(ref sender) = state.event_sender {
                let _ = sender.send_security_event(request.event, "api".to_string());
            }

            let response = ApiResponse::success("Event submitted successfully".to_string());
            Ok(JsonResponse(response))
        }
        Err(e) => {
            let error_response = ApiResponse::error(format!("Failed to submit event: {}", e));
            Err((StatusCode::INTERNAL_SERVER_ERROR, JsonResponse(error_response)))
        }
    }
}

/// Execute reasoning handler
pub async fn execute_reasoning(
    Extension(state): Extension<Arc<AppState>>,
    Json(_request): Json<ReasoningRequest>,
) -> Result<JsonResponse<ApiResponse<ReasoningResponse>>, (StatusCode, JsonResponse<ApiResponse<String>>)> {
    let start = Instant::now();

    match state.reasoner.reason().await {
        Ok(actions) => {
            let execution_time = start.elapsed();

            let response = ReasoningResponse {
                actions: actions.clone(),
                execution_time_ms: execution_time.as_millis() as u64,
                event_count: 0, // TODO: Get actual event count from reasoner
            };

            // Send reasoning result event if streaming is enabled
            #[cfg(feature = "streaming")]
            if let Some(ref sender) = state.event_sender {
                let _ = sender.send_reasoning_result(
                    actions,
                    execution_time.as_millis() as u64,
                    0, // TODO: Get actual event count
                );
            }

            Ok(JsonResponse(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_response = ApiResponse::error(format!("Reasoning failed: {}", e));
            Err((StatusCode::INTERNAL_SERVER_ERROR, JsonResponse(error_response)))
        }
    }
}

/// Query graph handler
pub async fn query_graph(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<GraphQueryRequest>,
) -> Result<JsonResponse<ApiResponse<GraphQueryResponse>>, (StatusCode, JsonResponse<ApiResponse<String>>)> {
    let store = state.reasoner.get_graph_store().await;
    let graph_store = store.read().await;

    let triples = graph_store.find_triples(
        request.subject.as_deref(),
        request.predicate.as_deref(),
        request.object.as_deref(),
    );

    let count = triples.len();
    let response = GraphQueryResponse {
        triples: triples.into_iter().map(|stored| stored.triple.clone()).collect(),
        count,
    };

    Ok(JsonResponse(ApiResponse::success(response)))
}

/// Get statistics handler
pub async fn get_stats(Extension(state): Extension<Arc<AppState>>) -> JsonResponse<ApiResponse<StatsResponse>> {
    let uptime = state.start_time.elapsed();

    // TODO: Get actual statistics from reasoner
    let response = StatsResponse {
        total_events: 0,
        total_actions: 0,
        uptime_seconds: uptime.as_secs(),
        memory_usage_mb: None, // TODO: Implement memory usage tracking
    };

    JsonResponse(ApiResponse::success(response))
}

/// Reset reasoner state handler
pub async fn reset_reasoner(
    Extension(_state): Extension<Arc<AppState>>,
) -> Result<JsonResponse<ApiResponse<String>>, (StatusCode, JsonResponse<ApiResponse<String>>)> {
    // TODO: Implement reset functionality - requires mutable access to reasoner
    let error_response = ApiResponse::error("Reset functionality not yet implemented".to_string());
    Err((StatusCode::NOT_IMPLEMENTED, JsonResponse(error_response)))
}

/// Add custom rule handler
pub async fn add_rule(
    Extension(_state): Extension<Arc<AppState>>,
    Json(_request): Json<AddRuleRequest>,
) -> Result<JsonResponse<ApiResponse<String>>, (StatusCode, JsonResponse<ApiResponse<String>>)> {
    // Note: This would require mutable access to reasoner, which needs design consideration
    // For now, return not implemented
    let error_response = ApiResponse::error("Adding custom rules not yet implemented".to_string());
    Err((StatusCode::NOT_IMPLEMENTED, JsonResponse(error_response)))
}

/// Get threat intelligence info handler
pub async fn get_threat_intel(
    Extension(state): Extension<Arc<AppState>>,
) -> JsonResponse<ApiResponse<ThreatIntelResponse>> {
    let threat_processor = state.threat_processor.read().await;
    let statistics = threat_processor.get_statistics();

    let response = ThreatIntelResponse {
        indicators_count: statistics.get("total_indicators").copied().unwrap_or(0),
        sources_count: statistics.get("sources").copied().unwrap_or(0),
        last_updated: chrono::Utc::now().timestamp(),
        statistics,
    };

    JsonResponse(ApiResponse::success(response))
}

/// Export threat indicators handler
pub async fn export_threat_indicators(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<JsonResponse<ApiResponse<String>>, (StatusCode, JsonResponse<ApiResponse<String>>)> {
    let threat_processor = state.threat_processor.read().await;

    match threat_processor.export_indicators() {
        Ok(json_data) => {
            Ok(JsonResponse(ApiResponse::success(json_data)))
        }
        Err(e) => {
            let error_response = ApiResponse::error(format!("Failed to export indicators: {}", e));
            Err((StatusCode::INTERNAL_SERVER_ERROR, JsonResponse(error_response)))
        }
    }
}

/// Import threat indicators handler
pub async fn import_threat_indicators(
    Extension(state): Extension<Arc<AppState>>,
    Json(json_data): Json<String>,
) -> Result<JsonResponse<ApiResponse<String>>, (StatusCode, JsonResponse<ApiResponse<String>>)> {
    let mut threat_processor = state.threat_processor.write().await;

    match threat_processor.import_indicators(&json_data) {
        Ok(_) => {
            let response = ApiResponse::success("Threat indicators imported successfully".to_string());
            Ok(JsonResponse(response))
        }
        Err(e) => {
            let error_response = ApiResponse::error(format!("Failed to import indicators: {}", e));
            Err((StatusCode::INTERNAL_SERVER_ERROR, JsonResponse(error_response)))
        }
    }
}

/// Monitoring: overall health
pub async fn monitoring_health(Extension(state): Extension<Arc<AppState>>) -> JsonResponse<HealthStatus> {
    let status = state.monitoring.get_overall_health().await;
    JsonResponse(status)
}

/// Monitoring: detailed checks
pub async fn monitoring_health_detailed(Extension(state): Extension<Arc<AppState>>) -> JsonResponse<Vec<HealthCheck>> {
    let checks = state.monitoring.run_health_checks().await;
    JsonResponse(checks)
}

/// Monitoring: system metrics
pub async fn monitoring_metrics(Extension(state): Extension<Arc<AppState>>) -> JsonResponse<SystemMetrics> {
    let metrics = state.monitoring.get_metrics().await;
    JsonResponse(metrics)
}
