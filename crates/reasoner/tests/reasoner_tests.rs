//! Tests for the reasoner crate

use reasoner_core::ReasonerEngine;
use reasoner_graph::model::{CyberEvent, SecurityAction, InferenceRule, Triple};
use reasoner_core::rules::RuleEngine;
use reasoner_core::inference::InferenceContext;
use reasoner_graph::store::GraphStore;
use tokio::test;

#[test]
async fn test_reasoner_engine_creation() {
    let reasoner = ReasonerEngine::new();

    // Engine should be created without errors
    assert!(reasoner.get_graph_store().await.is_ok());
}

#[test]
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

    // Check that event was added to graph
    let store = reasoner.get_graph_store().await.unwrap();
    let graph_store = store.read().await;
    let triples = graph_store.find_triples(None, None, None);
    assert!(!triples.is_empty());
}

#[test]
async fn test_reasoning_with_no_events() {
    let reasoner = ReasonerEngine::new();

    let actions = reasoner.reason().await.unwrap();
    // With no events and no rules, should return empty actions
    assert!(actions.is_empty());
}

#[test]
async fn test_rule_engine_creation() {
    let rule_engine = RuleEngine::new();
    assert!(rule_engine.get_rules().is_empty());
}

#[test]
fn test_rule_engine_add_rule() {
    let mut rule_engine = RuleEngine::new();

    let rule = InferenceRule {
        name: "test_rule".to_string(),
        description: "Test rule".to_string(),
        conditions: vec![],
        actions: vec![SecurityAction::Alert {
            severity: "test".to_string(),
            message: "Test alert".to_string(),
            details: serde_json::json!({"test": true}),
        }],
    };

    rule_engine.add_rule(rule.clone());

    let rules = rule_engine.get_rules();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "test_rule");
}

#[tokio::test]
async fn test_rule_evaluation_empty_conditions() {
    let mut store = GraphStore::new();
    let rule_engine = RuleEngine::new();

    // Add a rule with empty conditions (should always fire)
    let mut rule_engine = RuleEngine::new();
    let rule = InferenceRule {
        name: "always_fire".to_string(),
        description: "Always fires".to_string(),
        conditions: vec![],
        actions: vec![SecurityAction::Alert {
            severity: "info".to_string(),
            message: "Always fires".to_string(),
            details: serde_json::json!({}),
        }],
    };

    rule_engine.add_rule(rule);

    let context = InferenceContext::new();
    let actions = rule_engine.evaluate_rule(&store, &rule_engine.get_rules()[0], &context).await.unwrap();

    assert_eq!(actions.len(), 1);
    match &actions[0] {
        SecurityAction::Alert { severity, message, .. } => {
            assert_eq!(severity, "info");
            assert_eq!(message, "Always fires");
        }
        _ => panic!("Expected Alert action"),
    }
}

#[tokio::test]
async fn test_rule_evaluation_with_conditions() {
    let mut store = GraphStore::new();
    let rule_engine = RuleEngine::new();

    // Add test triple to store
    store.add_triple(Triple {
        subject: "https://example.com/test".to_string(),
        predicate: "https://example.com/ns/type".to_string(),
        object: "TestResource".to_string(),
    });

    // Add rule that checks for this triple
    let mut rule_engine = RuleEngine::new();
    let rule = InferenceRule {
        name: "test_condition".to_string(),
        description: "Test condition matching".to_string(),
        conditions: vec![Triple {
            subject: "https://example.com/test".to_string(),
            predicate: "https://example.com/ns/type".to_string(),
            object: "TestResource".to_string(),
        }],
        actions: vec![SecurityAction::Alert {
            severity: "warning".to_string(),
            message: "Condition matched".to_string(),
            details: serde_json::json!({"matched": true}),
        }],
    };

    rule_engine.add_rule(rule);

    let context = InferenceContext::new();
    let actions = rule_engine.evaluate_rule(&store, &rule_engine.get_rules()[0], &context).await.unwrap();

    assert_eq!(actions.len(), 1);
    match &actions[0] {
        SecurityAction::Alert { severity, message, .. } => {
            assert_eq!(severity, "warning");
            assert_eq!(message, "Condition matched");
        }
        _ => panic!("Expected Alert action"),
    }
}

#[tokio::test]
async fn test_rule_evaluation_no_match() {
    let mut store = GraphStore::new();
    let rule_engine = RuleEngine::new();

    // Add rule that checks for non-existent triple
    let mut rule_engine = RuleEngine::new();
    let rule = InferenceRule {
        name: "no_match".to_string(),
        description: "Should not match".to_string(),
        conditions: vec![Triple {
            subject: "https://example.com/nonexistent".to_string(),
            predicate: "https://example.com/ns/type".to_string(),
            object: "NonExistent".to_string(),
        }],
        actions: vec![SecurityAction::Alert {
            severity: "error".to_string(),
            message: "Should not fire".to_string(),
            details: serde_json::json!({"unexpected": true}),
        }],
    };

    rule_engine.add_rule(rule);

    let context = InferenceContext::new();
    let actions = rule_engine.evaluate_rule(&store, &rule_engine.get_rules()[0], &context).await.unwrap();

    // Should not fire because condition is not met
    assert!(actions.is_empty());
}

#[test]
fn test_inference_context_operations() {
    let mut context = InferenceContext::new();

    // Test variable binding
    context.bind_variable("test_var".to_string(), "test_value".to_string());
    assert!(context.is_bound("test_var"));
    assert_eq!(context.get_variable("test_var"), Some(&"test_value".to_string()));
    assert!(!context.is_bound("nonexistent"));

    // Test metadata
    context.set_metadata("test_key".to_string(), serde_json::json!("test_value"));
    assert_eq!(context.get_metadata("test_key"), Some(&serde_json::json!("test_value")));

    // Test reset
    context.reset();
    assert!(!context.is_bound("test_var"));
    assert!(context.get_metadata("test_key").is_none());
}

#[test]
fn test_inference_context_child() {
    let mut parent = InferenceContext::new();
    parent.bind_variable("parent_var".to_string(), "parent_value".to_string());

    let mut child = parent.create_child();
    child.bind_variable("child_var".to_string(), "child_value".to_string());

    // Child should inherit parent variables
    assert_eq!(child.get_variable("parent_var"), Some(&"parent_value".to_string()));
    assert_eq!(child.get_variable("child_var"), Some(&"child_value".to_string()));

    // Parent should not have child variables
    assert!(!parent.is_bound("child_var"));
}

#[tokio::test]
async fn test_malicious_ip_rule() {
    let reasoner = ReasonerEngine::new();

    // Add malicious IP connection event
    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "192.168.1.100".to_string(), // Known malicious IP
        port: 443,
        protocol: "tcp".to_string(),
        timestamp: 1640995200,
    };

    reasoner.add_event(event).await.unwrap();

    // Add malicious IP rule
    let rule = InferenceRule {
        name: "malicious_ip".to_string(),
        description: "Detect connections to malicious IPs".to_string(),
        conditions: vec![
            Triple {
                subject: "?connection".to_string(),
                predicate: "destIp".to_string(),
                object: "192.168.1.100".to_string(),
            },
        ],
        actions: vec![SecurityAction::BlockConnection {
            source_ip: "192.168.1.10".to_string(),
            dest_ip: "192.168.1.100".to_string(),
            reason: "Malicious IP detected".to_string(),
        }],
    };

    reasoner.add_rule(rule);

    let actions = reasoner.reason().await.unwrap();
    assert_eq!(actions.len(), 1);

    match &actions[0] {
        SecurityAction::BlockConnection { source_ip, dest_ip, reason } => {
            assert_eq!(source_ip, "192.168.1.10");
            assert_eq!(dest_ip, "192.168.1.100");
            assert_eq!(reason, "Malicious IP detected");
        }
        _ => panic!("Expected BlockConnection action"),
    }
}

#[tokio::test]
async fn test_multiple_events_reasoning() {
    let reasoner = ReasonerEngine::new();

    // Add multiple suspicious events
    let events = vec![
        CyberEvent::NetworkConnection {
            source_ip: "192.168.1.10".to_string(),
            dest_ip: "10.0.0.50".to_string(),
            port: 22,
            protocol: "tcp".to_string(),
            timestamp: 1640995200,
        },
        CyberEvent::ProcessExecution {
            process_id: 1234,
            parent_process_id: Some(1),
            command_line: "sudo rm -rf /".to_string(),
            user: "admin".to_string(),
            timestamp: 1640995260,
        },
    ];

    for event in events {
        reasoner.add_event(event).await.unwrap();
    }

    // Add rules for both scenarios
    let network_rule = InferenceRule {
        name: "suspicious_connection".to_string(),
        description: "Suspicious network connection".to_string(),
        conditions: vec![Triple {
            subject: "?connection".to_string(),
            predicate: "destIp".to_string(),
            object: "10.0.0.50".to_string(),
        }],
        actions: vec![SecurityAction::Alert {
            severity: "medium".to_string(),
            message: "Suspicious connection detected".to_string(),
            details: serde_json::json!({"ip": "10.0.0.50"}),
        }],
    };

    let process_rule = InferenceRule {
        name: "dangerous_command".to_string(),
        description: "Dangerous command execution".to_string(),
        conditions: vec![Triple {
            subject: "?process".to_string(),
            predicate: "commandLine".to_string(),
            object: "?cmd".to_string(),
        }],
        actions: vec![SecurityAction::Alert {
            severity: "high".to_string(),
            message: "Dangerous command executed".to_string(),
            details: serde_json::json!({"command": "?cmd"}),
        }],
    };

    reasoner.add_rule(network_rule);
    reasoner.add_rule(process_rule);

    let actions = reasoner.reason().await.unwrap();
    assert_eq!(actions.len(), 2);

    // Should have both alerts
    let severities: Vec<_> = actions.iter().map(|action| {
        match action {
            SecurityAction::Alert { severity, .. } => severity.clone(),
            _ => "unknown".to_string(),
        }
    }).collect();

    assert!(severities.contains(&"medium".to_string()));
    assert!(severities.contains(&"high".to_string()));
}

#[tokio::test]
async fn test_reasoner_reset() {
    let reasoner = ReasonerEngine::new();

    // Add event
    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "10.0.0.50".to_string(),
        port: 80,
        protocol: "tcp".to_string(),
        timestamp: 1640995200,
    };

    reasoner.add_event(event).await.unwrap();

    // Verify event exists
    let store = reasoner.get_graph_store().await.unwrap();
    let graph_store = store.read().await;
    assert!(!graph_store.find_triples(None, None, None).is_empty());

    // Reset
    reasoner.reset().await.unwrap();

    // Verify reset worked
    let store = reasoner.get_graph_store().await.unwrap();
    let graph_store = store.read().await;
    assert!(graph_store.find_triples(None, None, None).is_empty());
}
