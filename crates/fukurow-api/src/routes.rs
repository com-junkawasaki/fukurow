//! API route definitions

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use crate::{handlers::*, monitoring::routes::monitoring_routes};

/// Create the main API router
pub fn create_router(state: AppState) -> Router {
    let monitoring_router = monitoring_routes(state.monitoring.clone());

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

        // Apply middleware
        .layer(CorsLayer::permissive())
        .with_state(state)
        // Add monitoring routes as nested router
        .nest("/monitoring", monitoring_router)
}

/// API documentation routes (OpenAPI/Swagger)
pub fn create_docs_router() -> Router {
    Router::new()
        // TODO: Add OpenAPI/Swagger documentation routes
        // .route("/docs", get(serve_swagger_ui))
        // .route("/openapi.json", get(serve_openapi_spec))
}
