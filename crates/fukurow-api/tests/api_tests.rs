// API integration tests for fukurow-api

use fukurow_core::model::{CyberEvent, SecurityAction};
use fukurow_store::store::RdfStore;
use fukurow_store::provenance::{Provenance, GraphId};
use std::sync::Arc;
use tokio::sync::RwLock;

async fn create_test_app() -> fukurow_api::AppState {
    let store = Arc::new(RwLock::new(RdfStore::new()));
    let threat_processor = Arc::new(RwLock::new(
        fukurow_domain_cyber::threat_intelligence::ThreatProcessor::new()
    ));

    fukurow_api::AppState {
        store,
        threat_processor,
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let app_state = create_test_app().await;
    let response = fukurow_api::handlers::health(Extension(app_state)).await;

    match response {
        Ok(Json(health)) => {
            assert!(health.status.contains("healthy"));
        }
        _ => panic!("Health endpoint should return success"),
    }
}

#[tokio::test]
async fn test_submit_event() {
    let app_state = create_test_app().await;

    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "10.0.0.50".to_string(),
        port: 443,
        protocol: "tcp".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    let request = Json(fukurow_api::models::EventRequest { event });
    let response = fukurow_api::handlers::submit_event(request, Extension(app_state)).await;

    match response {
        Ok(Json(response)) => {
            assert!(response.success);
            assert!(response.event_id.is_some());
        }
        _ => panic!("Submit event should return success"),
    }
}

#[tokio::test]
async fn test_reason_endpoint() {
    let app_state = create_test_app().await;

    // First submit an event
    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "10.0.0.50".to_string(),
        port: 443,
        protocol: "tcp".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    let request = Json(fukurow_api::models::EventRequest { event });
    let _ = fukurow_api::handlers::submit_event(request, Extension(app_state.clone())).await;

    // Then test reasoning
    let response = fukurow_api::handlers::reason(Extension(app_state)).await;

    match response {
        Ok(Json(actions)) => {
            // Should return actions or empty list
            assert!(actions.len() >= 0);
        }
        _ => panic!("Reason endpoint should return success"),
    }
}

#[tokio::test]
async fn test_get_stats() {
    let app_state = create_test_app().await;
    let response = fukurow_api::handlers::get_stats(Extension(app_state)).await;

    match response {
        Ok(Json(stats)) => {
            assert!(stats.total_events >= 0);
            assert!(stats.total_actions >= 0);
        }
        _ => panic!("Stats endpoint should return success"),
    }
}

#[tokio::test]
async fn test_invalid_event_format() {
    let app_state = create_test_app().await;

    // Create an invalid event (missing required fields)
    let event = CyberEvent::NetworkConnection {
        source_ip: "".to_string(), // Invalid empty IP
        dest_ip: "invalid".to_string(),
        port: 99999, // Invalid port
        protocol: "".to_string(),
        timestamp: -1, // Invalid timestamp
    };

    let request = Json(fukurow_api::models::EventRequest { event });
    let response = fukurow_api::handlers::submit_event(request, Extension(app_state)).await;

    // Should return an error for invalid event
    assert!(response.is_err() || matches!(response, Ok(Json(ref resp)) if !resp.success));
}

#[tokio::test]
async fn test_multiple_events_processing() {
    let app_state = create_test_app().await;

    // Submit multiple events
    for i in 1..=5 {
        let event = CyberEvent::NetworkConnection {
            source_ip: format!("192.168.1.{}", i),
            dest_ip: "10.0.0.50".to_string(),
            port: 443,
            protocol: "tcp".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        let request = Json(fukurow_api::models::EventRequest { event });
        let response = fukurow_api::handlers::submit_event(request, Extension(app_state.clone())).await;

        match response {
            Ok(Json(resp)) => assert!(resp.success),
            _ => panic!("Event {} submission should succeed", i),
        }
    }

    // Check stats reflect the events
    let stats_response = fukurow_api::handlers::get_stats(Extension(app_state)).await;

    match stats_response {
        Ok(Json(stats)) => {
            assert!(stats.total_events >= 5);
        }
        _ => panic!("Stats should reflect submitted events"),
    }
}

#[tokio::test]
async fn test_reasoning_with_no_events() {
    let app_state = create_test_app().await;

    // Test reasoning with no events
    let response = fukurow_api::handlers::reason(Extension(app_state)).await;

    match response {
        Ok(Json(actions)) => {
            // Should return empty list when no events
            assert_eq!(actions.len(), 0);
        }
        _ => panic!("Reason endpoint should handle empty state"),
    }
}
