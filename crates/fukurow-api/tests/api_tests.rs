//! Tests for the api crate

use reasoner_api::models::*;
use reasoner_api::handlers::*;
use reasoner_graph::model::{CyberEvent, SecurityAction};
use axum::http::StatusCode;
use serde_json;

#[tokio::test]
async fn test_api_response_success() {
    let data = "test data".to_string();
    let response = ApiResponse::success(data);

    assert!(response.success);
    assert!(response.error.is_none());
    assert!(response.data.is_some());
    assert!(response.timestamp > 0);
}

#[tokio::test]
async fn test_api_response_error() {
    let error_msg = "test error".to_string();
    let response = ApiResponse::error(error_msg.clone());

    assert!(!response.success);
    assert!(response.data.is_none());
    assert_eq!(response.error, Some(error_msg));
    assert!(response.timestamp > 0);
}

#[tokio::test]
async fn test_cyber_event_request_creation() {
    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "10.0.0.50".to_string(),
        port: 443,
        protocol: "tcp".to_string(),
        timestamp: 1640995200,
    };

    let request = SubmitEventRequest { event };

    match &request.event {
        CyberEvent::NetworkConnection { source_ip, dest_ip, port, protocol, .. } => {
            assert_eq!(source_ip, "192.168.1.10");
            assert_eq!(dest_ip, "10.0.0.50");
            assert_eq!(*port, 443);
            assert_eq!(protocol, "tcp");
        }
        _ => panic!("Expected NetworkConnection event"),
    }
}

#[tokio::test]
async fn test_reasoning_request_creation() {
    let request = ReasoningRequest {
        include_details: Some(true),
    };

    assert_eq!(request.include_details, Some(true));

    let request_no_details = ReasoningRequest {
        include_details: None,
    };

    assert_eq!(request_no_details.include_details, None);
}

#[tokio::test]
async fn test_reasoning_response_creation() {
    let actions = vec![
        SecurityAction::Alert {
            severity: "high".to_string(),
            message: "Test alert".to_string(),
            details: serde_json::json!({"test": true}),
        },
        SecurityAction::BlockConnection {
            source_ip: "192.168.1.10".to_string(),
            dest_ip: "10.0.0.50".to_string(),
            reason: "Malicious activity".to_string(),
        },
    ];

    let response = ReasoningResponse {
        actions: actions.clone(),
        execution_time_ms: 150,
        event_count: 5,
    };

    assert_eq!(response.actions.len(), 2);
    assert_eq!(response.execution_time_ms, 150);
    assert_eq!(response.event_count, 5);

    // Verify actions are preserved
    match &response.actions[0] {
        SecurityAction::Alert { severity, message, .. } => {
            assert_eq!(severity, "high");
            assert_eq!(message, "Test alert");
        }
        _ => panic!("Expected Alert action"),
    }
}

#[tokio::test]
async fn test_graph_query_request() {
    let request = GraphQueryRequest {
        subject: Some("https://example.com/user/1".to_string()),
        predicate: Some("https://example.com/ns/name".to_string()),
        object: None,
        graph_name: Some("users".to_string()),
    };

    assert_eq!(request.subject, Some("https://example.com/user/1".to_string()));
    assert_eq!(request.predicate, Some("https://example.com/ns/name".to_string()));
    assert_eq!(request.object, None);
    assert_eq!(request.graph_name, Some("users".to_string()));
}

#[tokio::test]
async fn test_health_response() {
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: "0.1.0".to_string(),
        uptime_seconds: 3600,
    };

    assert_eq!(response.status, "healthy");
    assert_eq!(response.version, "0.1.0");
    assert_eq!(response.uptime_seconds, 3600);
}

#[tokio::test]
async fn test_stats_response() {
    let response = StatsResponse {
        total_events: 100,
        total_actions: 25,
        uptime_seconds: 7200,
        memory_usage_mb: Some(128.5),
    };

    assert_eq!(response.total_events, 100);
    assert_eq!(response.total_actions, 25);
    assert_eq!(response.uptime_seconds, 7200);
    assert_eq!(response.memory_usage_mb, Some(128.5));
}

#[tokio::test]
async fn test_api_error_types() {
    let invalid_req = ApiError::InvalidRequest("bad request".to_string());
    assert!(matches!(invalid_req, ApiError::InvalidRequest(_)));

    let event_err = ApiError::EventProcessingError("processing failed".to_string());
    assert!(matches!(event_err, ApiError::EventProcessingError(_)));

    let reasoning_err = ApiError::ReasoningError("reasoning failed".to_string());
    assert!(matches!(reasoning_err, ApiError::ReasoningError(_)));

    let graph_err = ApiError::GraphError("graph error".to_string());
    assert!(matches!(graph_err, ApiError::GraphError(_)));

    let internal_err = ApiError::InternalError("internal error".to_string());
    assert!(matches!(internal_err, ApiError::InternalError(_)));
}

#[test]
fn test_json_serialization_cyber_event() {
    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "10.0.0.50".to_string(),
        port: 443,
        protocol: "tcp".to_string(),
        timestamp: 1640995200,
    };

    let serialized = serde_json::to_string(&event).unwrap();
    let deserialized: CyberEvent = serde_json::from_str(&serialized).unwrap();

    match deserialized {
        CyberEvent::NetworkConnection { source_ip, dest_ip, port, protocol, timestamp } => {
            assert_eq!(source_ip, "192.168.1.10");
            assert_eq!(dest_ip, "10.0.0.50");
            assert_eq!(port, 443);
            assert_eq!(protocol, "tcp");
            assert_eq!(timestamp, 1640995200);
        }
        _ => panic!("Expected NetworkConnection"),
    }
}

#[test]
fn test_json_serialization_security_action() {
    let action = SecurityAction::BlockConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "10.0.0.50".to_string(),
        reason: "Malicious activity detected".to_string(),
    };

    let serialized = serde_json::to_string(&action).unwrap();
    let deserialized: SecurityAction = serde_json::from_str(&serialized).unwrap();

    match deserialized {
        SecurityAction::BlockConnection { source_ip, dest_ip, reason } => {
            assert_eq!(source_ip, "192.168.1.10");
            assert_eq!(dest_ip, "10.0.0.50");
            assert_eq!(reason, "Malicious activity detected");
        }
        _ => panic!("Expected BlockConnection"),
    }
}

#[test]
fn test_json_serialization_api_response() {
    let data = vec!["test".to_string(), "data".to_string()];
    let response = ApiResponse::success(data.clone());

    let serialized = serde_json::to_string(&response).unwrap();
    let deserialized: ApiResponse<Vec<String>> = serde_json::from_str(&serialized).unwrap();

    assert!(deserialized.success);
    assert!(deserialized.error.is_none());
    assert_eq!(deserialized.data, Some(data));
}

#[test]
fn test_json_serialization_error_response() {
    let error_msg = "Test error message".to_string();
    let response = ApiResponse::<String>::error(error_msg.clone());

    let serialized = serde_json::to_string(&response).unwrap();
    let deserialized: ApiResponse<String> = serde_json::from_str(&serialized).unwrap();

    assert!(!deserialized.success);
    assert!(deserialized.data.is_none());
    assert_eq!(deserialized.error, Some(error_msg));
}

#[tokio::test]
async fn test_app_state_creation() {
    use reasoner_core::ReasonerEngine;
    use rules_cyber::threat_intelligence::ThreatProcessor;

    let reasoner = ReasonerEngine::new();
    let threat_processor = ThreatProcessor::new();

    let app_state = AppState {
        reasoner: std::sync::Arc::new(reasoner),
        threat_processor: std::sync::Arc::new(tokio::sync::RwLock::new(threat_processor)),
        start_time: std::time::Instant::now(),
    };

    // Should be able to get stores
    let _graph_store = app_state.reasoner.get_graph_store().await.unwrap();
    let _threat_processor = app_state.threat_processor.read().await;
}

#[test]
fn test_inferencerule_serialization() {
    use reasoner_graph::model::InferenceRule;
    use reasoner_graph::model::Triple;

    let rule = InferenceRule {
        name: "test_rule".to_string(),
        description: "Test inference rule".to_string(),
        conditions: vec![Triple {
            subject: "?event".to_string(),
            predicate: "type".to_string(),
            object: "NetworkConnection".to_string(),
        }],
        actions: vec![SecurityAction::Alert {
            severity: "medium".to_string(),
            message: "Test alert".to_string(),
            details: serde_json::json!({"test": true}),
        }],
    };

    let serialized = serde_json::to_string(&rule).unwrap();
    let deserialized: InferenceRule = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.name, "test_rule");
    assert_eq!(deserialized.description, "Test inference rule");
    assert_eq!(deserialized.conditions.len(), 1);
    assert_eq!(deserialized.actions.len(), 1);
}

#[test]
fn test_add_rule_request() {
    use reasoner_graph::model::InferenceRule;

    let rule = InferenceRule {
        name: "test_rule".to_string(),
        description: "Test rule".to_string(),
        conditions: vec![],
        actions: vec![SecurityAction::Alert {
            severity: "info".to_string(),
            message: "Test".to_string(),
            details: serde_json::json!({}),
        }],
    };

    let request = AddRuleRequest { rule: rule.clone() };

    assert_eq!(request.rule.name, "test_rule");
    assert_eq!(request.rule.actions.len(), 1);
}

#[test]
fn test_rules_response() {
    use reasoner_graph::model::InferenceRule;

    let rules = vec![
        InferenceRule {
            name: "rule1".to_string(),
            description: "First rule".to_string(),
            conditions: vec![],
            actions: vec![],
        },
        InferenceRule {
            name: "rule2".to_string(),
            description: "Second rule".to_string(),
            conditions: vec![],
            actions: vec![],
        },
    ];

    let response = RulesResponse {
        rules: rules.clone(),
        count: rules.len(),
    };

    assert_eq!(response.rules.len(), 2);
    assert_eq!(response.count, 2);
    assert_eq!(response.rules[0].name, "rule1");
    assert_eq!(response.rules[1].name, "rule2");
}

#[test]
fn test_threat_intel_response() {
    use std::collections::HashMap;

    let mut statistics = HashMap::new();
    statistics.insert("total_indicators".to_string(), 150);
    statistics.insert("ip_indicators".to_string(), 100);
    statistics.insert("domain_indicators".to_string(), 50);

    let response = ThreatIntelResponse {
        indicators_count: 150,
        sources_count: 5,
        last_updated: 1640995200,
        statistics,
    };

    assert_eq!(response.indicators_count, 150);
    assert_eq!(response.sources_count, 5);
    assert_eq!(response.last_updated, 1640995200);
    assert_eq!(response.statistics.get("total_indicators"), Some(&150));
    assert_eq!(response.statistics.get("ip_indicators"), Some(&100));
    assert_eq!(response.statistics.get("domain_indicators"), Some(&50));
}

#[test]
fn test_api_error_from_reasoner_error() {
    use reasoner_core::ReasonerError;
    use std::fmt;

    // Create a mock error (since we can't easily create real ones)
    impl fmt::Display for ReasonerError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Mock reasoner error")
        }
    }

    // Test that ApiError can be created from ReasonerError
    // (The actual conversion is tested at the boundary)
    let reasoner_err = ReasonerError::RuleError("test".to_string());
    let api_err: ApiError = reasoner_err.into();

    match api_err {
        ApiError::ReasoningError(_) => {} // Expected
        _ => panic!("Expected ReasoningError"),
    }
}

#[test]
fn test_api_error_from_anyhow() {
    use anyhow::anyhow;

    let anyhow_err = anyhow!("test error");
    let api_err: ApiError = anyhow_err.into();

    match api_err {
        ApiError::InternalError(_) => {} // Expected
        _ => panic!("Expected InternalError"),
    }
}

#[test]
fn test_request_validation() {
    // Test that invalid requests are properly structured
    let invalid_event_request = SubmitEventRequest {
        event: CyberEvent::NetworkConnection {
            source_ip: "".to_string(), // Invalid empty IP
            dest_ip: "invalid".to_string(),
            port: 99999, // Invalid port
            protocol: "invalid_protocol".to_string(),
            timestamp: -1, // Invalid timestamp
        },
    };

    // Should serialize/deserialize without panicking
    let serialized = serde_json::to_string(&invalid_event_request).unwrap();
    let deserialized: SubmitEventRequest = serde_json::from_str(&serialized).unwrap();

    match deserialized.event {
        CyberEvent::NetworkConnection { source_ip, dest_ip, port, protocol, timestamp } => {
            assert_eq!(source_ip, "");
            assert_eq!(dest_ip, "invalid");
            assert_eq!(port, 99999);
            assert_eq!(protocol, "invalid_protocol");
            assert_eq!(timestamp, -1);
        }
        _ => panic!("Expected NetworkConnection"),
    }
}

#[test]
fn test_response_consistency() {
    // Test that responses maintain data integrity through serialization
    let original_actions = vec![
        SecurityAction::IsolateHost {
            host_ip: "192.168.1.100".to_string(),
            reason: "Compromised host".to_string(),
        },
        SecurityAction::TerminateProcess {
            process_id: 1234,
            reason: "Malicious process".to_string(),
        },
    ];

    let original_response = ReasoningResponse {
        actions: original_actions.clone(),
        execution_time_ms: 250,
        event_count: 10,
    };

    let api_response = ApiResponse::success(original_response);

    // Serialize and deserialize
    let serialized = serde_json::to_string(&api_response).unwrap();
    let deserialized: ApiResponse<ReasoningResponse> = serde_json::from_str(&serialized).unwrap();

    assert!(deserialized.success);
    assert!(deserialized.error.is_none());

    let response_data = deserialized.data.unwrap();
    assert_eq!(response_data.actions.len(), 2);
    assert_eq!(response_data.execution_time_ms, 250);
    assert_eq!(response_data.event_count, 10);

    // Verify actions are preserved
    match &response_data.actions[0] {
        SecurityAction::IsolateHost { host_ip, reason } => {
            assert_eq!(host_ip, "192.168.1.100");
            assert_eq!(reason, "Compromised host");
        }
        _ => panic!("Expected IsolateHost"),
    }
}
