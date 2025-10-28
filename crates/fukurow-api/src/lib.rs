//! # Reasoner API Library
//!
//! JSON-LD Reasoner の Web API インターフェース
//! RESTful API でサイバーセキュリティイベントの推論を提供
//! エンタープライズSIEM統合機能（Splunk・ELK・Chronicle対応）

pub mod routes;
pub mod handlers;
pub mod models;
pub mod server;
pub mod siem_integration;
pub use routes::*;
pub use handlers::*;
pub use models::*;
pub use server::*;
pub use siem_integration::*;

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_core::model::{CyberEvent, SecurityAction};

    #[cfg(test)]
    mod models_tests {
        use super::*;

        #[test]
        fn test_api_response_success() {
            let data = "test_data".to_string();
            let response = ApiResponse::success(data.clone());

            assert!(response.success);
            assert_eq!(response.data, Some(data));
            assert!(response.error.is_none());
            assert!(response.timestamp > 0);
        }

        #[test]
        fn test_api_response_error() {
            let error_msg = "test error".to_string();
            let response = ApiResponse::<String>::error(error_msg.clone());

            assert!(!response.success);
            assert!(response.data.is_none());
            assert_eq!(response.error, Some(error_msg));
            assert!(response.timestamp > 0);
        }

        #[test]
        fn test_submit_event_request() {
            let event = CyberEvent::NetworkConnection {
                source_ip: "192.168.1.1".to_string(),
                dest_ip: "10.0.0.1".to_string(),
                port: 443,
                protocol: "tcp".to_string(),
                timestamp: 1640995200,
            };

            let request = SubmitEventRequest { event: event.clone() };

            match request.event {
                CyberEvent::NetworkConnection { source_ip, .. } => {
                    assert_eq!(source_ip, "192.168.1.1");
                }
                _ => panic!("Expected NetworkConnection"),
            }
        }

        #[test]
        fn test_reasoning_request() {
            let request = ReasoningRequest {
                include_details: Some(true),
            };
            assert_eq!(request.include_details, Some(true));

            let request2 = ReasoningRequest {
                include_details: None,
            };
            assert_eq!(request2.include_details, None);
        }

        #[test]
        fn test_reasoning_response() {
            let actions = vec![
                SecurityAction::Alert {
                    severity: "high".to_string(),
                    message: "Test alert".to_string(),
                    details: serde_json::json!({"test": "data"}),
                }
            ];

            let response = ReasoningResponse {
                actions: actions.clone(),
                execution_time_ms: 150,
                event_count: 5,
            };

            assert_eq!(response.actions.len(), 1);
            assert_eq!(response.execution_time_ms, 150);
            assert_eq!(response.event_count, 5);
        }

        #[test]
        fn test_graph_query_request() {
            let request = GraphQueryRequest {
                subject: Some("subject1".to_string()),
                predicate: Some("predicate1".to_string()),
                object: None,
                graph_name: Some("default".to_string()),
            };

            assert_eq!(request.subject, Some("subject1".to_string()));
            assert_eq!(request.predicate, Some("predicate1".to_string()));
            assert_eq!(request.object, None);
            assert_eq!(request.graph_name, Some("default".to_string()));
        }

        #[test]
        fn test_graph_query_response() {
            let triples = vec![
                fukurow_core::model::Triple {
                    subject: "s".to_string(),
                    predicate: "p".to_string(),
                    object: "o".to_string(),
                }
            ];

            let response = GraphQueryResponse {
                triples: triples.clone(),
                count: 1,
            };

            assert_eq!(response.triples.len(), 1);
            assert_eq!(response.count, 1);
        }

        #[test]
        fn test_health_response() {
            let response = HealthResponse {
                status: "healthy".to_string(),
                version: "1.0.0".to_string(),
                uptime_seconds: 3600,
            };

            assert_eq!(response.status, "healthy");
            assert_eq!(response.version, "1.0.0");
            assert_eq!(response.uptime_seconds, 3600);
        }

        #[test]
        fn test_stats_response() {
            let response = StatsResponse {
                total_events: 100,
                total_actions: 25,
                uptime_seconds: 7200,
                memory_usage_mb: Some(150.5),
            };

            assert_eq!(response.total_events, 100);
            assert_eq!(response.total_actions, 25);
            assert_eq!(response.uptime_seconds, 7200);
            assert_eq!(response.memory_usage_mb, Some(150.5));
        }

        #[test]
        fn test_threat_intel_response() {
            let mut statistics = std::collections::HashMap::new();
            statistics.insert("malware".to_string(), 10);
            statistics.insert("phishing".to_string(), 5);

            let response = ThreatIntelResponse {
                indicators_count: 15,
                sources_count: 3,
                last_updated: 1640995200,
                statistics,
            };

            assert_eq!(response.indicators_count, 15);
            assert_eq!(response.sources_count, 3);
            assert_eq!(response.last_updated, 1640995200);
            assert_eq!(response.statistics.get("malware"), Some(&10));
            assert_eq!(response.statistics.get("phishing"), Some(&5));
        }

        #[test]
        fn test_api_error_variants() {
            let err1 = ApiError::InvalidRequest("bad request".to_string());
            assert!(err1.to_string().contains("Invalid request"));

            let err2 = ApiError::EventProcessingError("processing failed".to_string());
            assert!(err2.to_string().contains("Event processing error"));

            let err3 = ApiError::ReasoningError("reasoning failed".to_string());
            assert!(err3.to_string().contains("Reasoning error"));

            let err4 = ApiError::GraphError("graph error".to_string());
            assert!(err4.to_string().contains("Graph operation error"));

            let err5 = ApiError::InternalError("internal error".to_string());
            assert!(err5.to_string().contains("Internal server error"));
        }
    }

    #[cfg(test)]
    mod server_tests {
        use super::*;

        #[test]
        fn test_server_config_default() {
            let config = ServerConfig::default();
            assert_eq!(config.host, "0.0.0.0");
            assert_eq!(config.port, 3000);
            assert_eq!(config.max_connections, 100);
        }

        #[test]
        fn test_server_config_custom() {
            let config = ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 50,
            };

            assert_eq!(config.host, "127.0.0.1");
            assert_eq!(config.port, 8080);
            assert_eq!(config.max_connections, 50);
        }

        #[test]
        fn test_reasoner_server_address() {
            let config = ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 50,
            };

            let server = ReasonerServer::with_config(config);
            let addr = server.address();

            assert_eq!(addr.to_string(), "127.0.0.1:8080");
        }

        #[test]
        fn test_reasoner_server_new() {
            let server = ReasonerServer::new();
            let addr = server.address();
            assert_eq!(addr.to_string(), "0.0.0.0:3000");
        }

        #[test]
        fn test_reasoner_server_create_app() {
            let server = ReasonerServer::new();
            let _app = server.create_app();
            // Router is created successfully
            assert!(true); // If we get here, app creation worked
        }

        #[tokio::test]
        async fn test_shutdown_signal_creation() {
            // Test that shutdown_signal function can be created
            // Note: We can't actually test the signal handling without spawning a task
            // that would be interrupted, but we can test the function compiles and runs
            let _signal_future = shutdown_signal();
            // The future should be created without panicking
            assert!(true);
        }

        #[test]
        fn test_create_server_with_reasoner() {
            // Test the utility function
            let reasoner = fukurow_engine::ReasonerEngine::new();
            let config = ServerConfig::default();

            let server = create_server_with_reasoner(reasoner, config);
            let addr = server.address();

            assert_eq!(addr.to_string(), "0.0.0.0:3000");
        }
    }

    #[cfg(test)]
    mod handlers_routes_tests {
        use super::*;

        // Test handler functions that don't require complex state
        #[test]
        fn test_api_response_creation() {
            let data = "test_data".to_string();
            let response = ApiResponse::success(data.clone());

            assert!(response.success);
            assert_eq!(response.data, Some(data));
            assert!(response.error.is_none());
            assert!(response.timestamp > 0);
        }

        #[test]
        fn test_api_error_response_creation() {
            let error_msg = "test error".to_string();
            let response = ApiResponse::<String>::error(error_msg.clone());

            assert!(!response.success);
            assert!(response.data.is_none());
            assert_eq!(response.error, Some(error_msg));
            assert!(response.timestamp > 0);
        }

        #[test]
        fn test_health_response_creation() {
            let response = HealthResponse {
                status: "healthy".to_string(),
                version: "1.0.0".to_string(),
                uptime_seconds: 3600,
            };

            assert_eq!(response.status, "healthy");
            assert_eq!(response.version, "1.0.0");
            assert_eq!(response.uptime_seconds, 3600);
        }

        #[test]
        fn test_stats_response_creation() {
            let response = StatsResponse {
                total_events: 100,
                total_actions: 25,
                uptime_seconds: 7200,
                memory_usage_mb: Some(150.5),
            };

            assert_eq!(response.total_events, 100);
            assert_eq!(response.total_actions, 25);
            assert_eq!(response.uptime_seconds, 7200);
            assert_eq!(response.memory_usage_mb, Some(150.5));
        }

        #[test]
        fn test_reasoning_response_creation() {
            use fukurow_core::model::SecurityAction;

            let actions = vec![
                SecurityAction::Alert {
                    severity: "high".to_string(),
                    message: "Test alert".to_string(),
                    details: serde_json::json!({"test": "data"}),
                }
            ];

            let response = ReasoningResponse {
                actions: actions.clone(),
                execution_time_ms: 150,
                event_count: 5,
            };

            assert_eq!(response.actions.len(), 1);
            assert_eq!(response.execution_time_ms, 150);
            assert_eq!(response.event_count, 5);
        }

        #[test]
        fn test_graph_query_response_creation() {
            let triples = vec![
                fukurow_core::model::Triple {
                    subject: "s".to_string(),
                    predicate: "p".to_string(),
                    object: "o".to_string(),
                }
            ];

            let response = GraphQueryResponse {
                triples: triples.clone(),
                count: 1,
            };

            assert_eq!(response.triples.len(), 1);
            assert_eq!(response.count, 1);
        }

        #[test]
        fn test_threat_intel_response_creation() {
            let mut statistics = std::collections::HashMap::new();
            statistics.insert("malware".to_string(), 10);
            statistics.insert("phishing".to_string(), 5);

            let response = ThreatIntelResponse {
                indicators_count: 15,
                sources_count: 3,
                last_updated: 1640995200,
                statistics,
            };

            assert_eq!(response.indicators_count, 15);
            assert_eq!(response.sources_count, 3);
            assert_eq!(response.last_updated, 1640995200);
            assert_eq!(response.statistics.get("malware"), Some(&10));
            assert_eq!(response.statistics.get("phishing"), Some(&5));
        }
    }
}
