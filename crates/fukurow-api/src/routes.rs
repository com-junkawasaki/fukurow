//! API route definitions

use axum::{
    routing::{get, post},
    Router,
    extract::Extension,
};
use tower_http::cors::CorsLayer;
use std::sync::Arc;
use crate::handlers::*;
/// Create the main API router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health and status routes
        .route("/health", get(health_check))
        .route("/stats", get(get_stats))

        // Event management routes
        .route("/events", post(submit_event))

        // Reasoning routes
        .route("/reason", post(execute_reasoning))
        .route("/reason/reset", post(reset_reasoner))

        // Graph query routes
        .route("/graph/query", post(query_graph))

        // Rule management routes (future)
        .route("/rules", post(add_rule))

        // Threat intelligence routes
        .route("/threat-intel", get(get_threat_intel))
        .route("/threat-intel/export", get(export_threat_indicators))
        .route("/threat-intel/import", post(import_threat_indicators))

        // Monitoring routes (bound to AppState)
        .route("/monitoring/health", get(monitoring_health))
        .route("/monitoring/health/detailed", get(monitoring_health_detailed))
        .route("/monitoring/metrics", get(monitoring_metrics))

        // Apply middleware
        .layer(CorsLayer::permissive())
        .layer(Extension(state))
}

/// API documentation routes (OpenAPI/Swagger)
pub fn create_docs_router() -> Router {
    Router::new()
        // TODO: Add OpenAPI/Swagger documentation routes
        // .route("/docs", get(serve_swagger_ui))
        // .route("/openapi.json", get(serve_openapi_spec))
}
