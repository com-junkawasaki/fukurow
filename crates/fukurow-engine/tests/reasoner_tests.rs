//! Tests for the fukurow-engine crate

use fukurow_engine::*;
use fukurow_core::model::{CyberEvent, SecurityAction, Triple};
use fukurow_store::store::RdfStore;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_reasoner_engine_creation() {
    let reasoner = ReasonerEngine::new();

    // Engine should be created without errors
    let store = reasoner.get_graph_store().await;
    // If we reach here, creation succeeded
    assert!(true);
}

#[tokio::test]
async fn test_add_event() {
    let reasoner = ReasonerEngine::new();

    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "10.0.0.50".to_string(),
        port: 443,
        protocol: "tcp".to_string(),
        timestamp: 1640995200,
    };

    let result = reasoner.add_event(event).await;
    assert!(result.is_ok());

    // Check that the event was added to the graph
    let store = reasoner.get_graph_store().await;
    let store_read = store.read().await;
    let triples = store_read.find_triples(None, None, None);
    assert!(!triples.is_empty());
}

#[tokio::test]
async fn test_reasoning_engine_creation() {
    let engine = ReasoningEngine::new();
    assert!(true); // Engine should be created without errors
}

#[tokio::test]
async fn test_reasoning_engine_default_options() {
    let options = ProcessingOptions::default();
    assert_eq!(options.max_iterations, 10);
    assert!(options.enable_validation);
    assert!(options.enable_inference);
    assert!(options.enable_rdfs_inference);
    assert_eq!(options.timeout_ms, Some(5000));
}

#[tokio::test]
async fn test_empty_reasoning() {
    let engine = ReasoningEngine::new();
    let store = RdfStore::new();

    let result = engine.process(&store).await;
    assert!(result.is_ok());

    let engine_result = result.unwrap();
    assert_eq!(engine_result.inferred_triples.len(), 0);
    assert_eq!(engine_result.actions.len(), 0);
    assert_eq!(engine_result.violations.len(), 0);
}

#[tokio::test]
async fn test_engine_result_creation() {
    let result = EngineResult {
        inferred_triples: vec![
            Triple {
                subject: "test".to_string(),
                predicate: "type".to_string(),
                object: "Test".to_string(),
            }
        ],
        actions: vec![
            SecurityAction::Alert {
                severity: "high".to_string(),
                message: "Test alert".to_string(),
                details: serde_json::json!({"test": true}),
            }
        ],
        violations: vec![],
        stats: ProcessingStats {
            rules_applied: 1,
            triples_processed: 10,
            execution_time_ms: 150,
            memory_used_kb: Some(1024),
        },
    };

    assert_eq!(result.inferred_triples.len(), 1);
    assert_eq!(result.actions.len(), 1);
    assert_eq!(result.violations.len(), 0);
    assert_eq!(result.stats.rules_applied, 1);
    assert_eq!(result.stats.triples_processed, 10);
    assert_eq!(result.stats.execution_time_ms, 150);
    assert_eq!(result.stats.memory_used_kb, Some(1024));
}

#[tokio::test]
async fn test_processing_stats_creation() {
    let stats = ProcessingStats {
        rules_applied: 5,
        triples_processed: 100,
        execution_time_ms: 200,
        memory_used_kb: None,
    };

    assert_eq!(stats.rules_applied, 5);
    assert_eq!(stats.triples_processed, 100);
    assert_eq!(stats.execution_time_ms, 200);
    assert_eq!(stats.memory_used_kb, None);
}

#[test]
fn test_engine_error_variants() {
    let rule_err = EngineError::RuleError(fukurow_rules::RuleError::ConfigurationError("test".to_string()));
    assert!(rule_err.to_string().contains("Rule execution failed"));

    let rdfs_err = EngineError::RdfsError(fukurow_rdfs::RdfsError::InferenceError("test".to_string()));
    assert!(rdfs_err.to_string().contains("RDFS reasoning failed"));

    let timeout_err = EngineError::TimeoutError(5000);
    assert!(timeout_err.to_string().contains("5000ms"));

    let iteration_err = EngineError::IterationLimitError(10);
    assert!(iteration_err.to_string().contains("10"));

    let internal_err = EngineError::InternalError("test error".to_string());
    assert!(internal_err.to_string().contains("test error"));
}
