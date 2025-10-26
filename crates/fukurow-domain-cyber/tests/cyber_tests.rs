//! Tests for the rules-cyber crate

use rules_cyber::detectors::{MaliciousIpDetector, LateralMovementDetector, PrivilegeEscalationDetector};
use rules_cyber::patterns::{PatternDetector, AnomalyDetector};
use rules_cyber::threat_intelligence::{ThreatProcessor, ThreatIndicator, IndicatorType, ThreatSource};
use reasoner_graph::model::{CyberEvent, SecurityAction};

#[test]
fn test_malicious_ip_detector() {
    let detector = MaliciousIpDetector::new();

    // Test known malicious IPs
    assert!(detector.is_malicious_ip("192.168.1.100"));
    assert!(detector.is_malicious_ip("10.0.0.50"));
    assert!(!detector.is_malicious_ip("8.8.8.8"));

    // Test pattern matching
    assert!(detector.is_malicious_ip("192.168.1.200")); // Pattern match
    assert!(!detector.is_malicious_ip("172.16.0.1")); // No pattern match
}

#[test]
fn test_malicious_ip_detector_rule() {
    let detector = MaliciousIpDetector::new();
    let rule = detector.create_rule();

    assert_eq!(rule.name, "advanced_malicious_ip_detection");
    assert!(rule.conditions.len() > 0);
    assert!(rule.actions.len() > 0);

    // Check that actions include BlockConnection
    let has_block_action = rule.actions.iter().any(|action| {
        matches!(action, SecurityAction::BlockConnection { .. })
    });
    assert!(has_block_action);
}

#[test]
fn test_lateral_movement_detector() {
    let detector = LateralMovementDetector::new();

    // Create test events simulating lateral movement
    let events = vec![
        CyberEvent::UserLogin {
            user: "alice".to_string(),
            source_ip: "192.168.1.10".to_string(),
            success: true,
            timestamp: 1640995200,
        },
        CyberEvent::UserLogin {
            user: "alice".to_string(),
            source_ip: "192.168.1.20".to_string(),
            success: true,
            timestamp: 1640995260, // 1 minute later
        },
        CyberEvent::UserLogin {
            user: "alice".to_string(),
            source_ip: "192.168.1.30".to_string(),
            success: true,
            timestamp: 1640995320, // Another minute later
        },
    ];

    let actions = detector.detect_lateral_movement(&events);

    // Should detect lateral movement with 3 different IPs
    assert!(!actions.is_empty());
    let has_alert = actions.iter().any(|action| {
        matches!(action, SecurityAction::Alert { .. })
    });
    assert!(has_alert);
}

#[test]
fn test_lateral_movement_detector_no_movement() {
    let detector = LateralMovementDetector::new();

    // Single IP login - no lateral movement
    let events = vec![
        CyberEvent::UserLogin {
            user: "bob".to_string(),
            source_ip: "192.168.1.10".to_string(),
            success: true,
            timestamp: 1640995200,
        },
    ];

    let actions = detector.detect_lateral_movement(&events);
    assert!(actions.is_empty());
}

#[test]
fn test_privilege_escalation_detector() {
    let detector = PrivilegeEscalationDetector::new();

    // Test dangerous commands
    assert!(detector.is_dangerous_command("sudo rm -rf /"));
    assert!(detector.is_dangerous_command("chmod 777 /etc/passwd"));
    assert!(detector.is_dangerous_command("netcat -l 8080"));
    assert!(!detector.is_dangerous_command("ls -la"));
    assert!(!detector.is_dangerous_command("cat /etc/hosts"));
}

#[test]
fn test_privilege_escalation_detector_rule() {
    let detector = PrivilegeEscalationDetector::new();
    let rule = detector.create_rule();

    assert_eq!(rule.name, "privilege_escalation_detection");
    assert!(rule.conditions.len() > 0);
    assert!(rule.actions.len() > 0);

    // Check that actions include IsolateHost and RevokePrivileges
    let has_isolate = rule.actions.iter().any(|action| {
        matches!(action, SecurityAction::IsolateHost { .. })
    });
    let has_revoke = rule.actions.iter().any(|action| {
        matches!(action, SecurityAction::RevokePrivileges { .. })
    });

    assert!(has_isolate);
    assert!(has_revoke);
}

#[test]
fn test_pattern_detector() {
    let detector = PatternDetector::new();

    // Test ransomware pattern
    let ransomware_event = CyberEvent::ProcessExecution {
        process_id: 1234,
        parent_process_id: None,
        command_line: "encrypt.exe --bitcoin-wallet abc123".to_string(),
        user: "user".to_string(),
        timestamp: 1640995200,
    };

    let actions = detector.match_patterns(&[ransomware_event]);
    assert!(!actions.is_empty());

    // Check for critical severity alert
    let has_critical_alert = actions.iter().any(|action| {
        match action {
            SecurityAction::Alert { severity, .. } => severity == "critical",
            _ => false,
        }
    });
    assert!(has_critical_alert);
}

#[test]
fn test_pattern_detector_no_match() {
    let detector = PatternDetector::new();

    // Normal process execution
    let normal_event = CyberEvent::ProcessExecution {
        process_id: 1234,
        parent_process_id: None,
        command_line: "notepad.exe".to_string(),
        user: "user".to_string(),
        timestamp: 1640995200,
    };

    let actions = detector.match_patterns(&[normal_event]);
    assert!(actions.is_empty());
}

#[test]
fn test_anomaly_detector() {
    let mut detector = AnomalyDetector::new();

    // Set baseline
    detector.update_baseline("cpu_usage".to_string(), 50.0);

    // Normal value - no anomaly
    let normal_result = detector.detect_anomaly("cpu_usage", 55.0, 0.2);
    assert!(normal_result.is_none());

    // Anomalous value - should detect
    let anomaly_result = detector.detect_anomaly("cpu_usage", 80.0, 0.2);
    assert!(anomaly_result.is_some());

    if let Some(SecurityAction::Alert { severity, message, .. }) = anomaly_result {
        assert_eq!(severity, "medium");
        assert!(message.contains("Anomaly detected"));
    } else {
        panic!("Expected Alert action");
    }
}

#[test]
fn test_threat_processor_creation() {
    let processor = ThreatProcessor::new();
    let stats = processor.get_statistics();

    // Should have sample data loaded
    assert!(stats.get("total_indicators").unwrap() > &0);
    assert!(stats.get("ip_indicators").unwrap() > &0);
    assert!(stats.get("domain_indicators").unwrap() > &0);
}

#[test]
fn test_threat_processor_known_threat() {
    let processor = ThreatProcessor::new();

    // Test known malicious IP
    let threat_result = processor.process_event("192.168.1.100", IndicatorType::IpAddress);
    assert!(threat_result.is_some());
    assert!(threat_result.unwrap().contains("malware_c2"));

    // Test unknown IP
    let safe_result = processor.process_event("8.8.8.8", IndicatorType::IpAddress);
    assert!(safe_result.is_none());
}

#[test]
fn test_threat_processor_known_domain() {
    let processor = ThreatProcessor::new();

    // Test known malicious domain
    let threat_result = processor.process_event("malicious-site.example.com", IndicatorType::Domain);
    assert!(threat_result.is_some());
    assert!(threat_result.unwrap().contains("phishing"));

    // Test unknown domain
    let safe_result = processor.process_event("google.com", IndicatorType::Domain);
    assert!(safe_result.is_none());
}

#[test]
fn test_threat_processor_statistics() {
    let processor = ThreatProcessor::new();
    let stats = processor.get_statistics();

    assert!(stats.contains_key("total_indicators"));
    assert!(stats.contains_key("ip_indicators"));
    assert!(stats.contains_key("domain_indicators"));
    assert!(stats.contains_key("sources"));

    // All counts should be non-negative
    for count in stats.values() {
        assert!(*count >= 0);
    }
}

#[test]
fn test_threat_processor_export_import() {
    let processor = ThreatProcessor::new();

    // Export indicators
    let export_result = processor.export_indicators();
    assert!(export_result.is_ok());

    let json_data = export_result.unwrap();

    // Create new processor and import
    let mut new_processor = ThreatProcessor::new();
    let import_result = new_processor.import_indicators(&json_data);
    assert!(import_result.is_ok());

    // Statistics should be similar (may have duplicates but should work)
    let original_stats = processor.get_statistics();
    let imported_stats = new_processor.get_statistics();
    assert!(imported_stats.get("total_indicators").unwrap() >= original_stats.get("total_indicators").unwrap());
}

#[test]
fn test_threat_processor_filtering() {
    let processor = ThreatProcessor::new();

    // Get all IP indicators
    let ip_indicators = processor.feed().get_indicators_by_type(IndicatorType::IpAddress);
    assert!(!ip_indicators.is_empty());

    // Get indicators by threat type
    let malware_indicators = processor.feed().get_indicators_by_threat_type("malware_c2");
    assert!(!malware_indicators.is_empty());

    // Get phishing indicators
    let phishing_indicators = processor.feed().get_indicators_by_threat_type("phishing");
    assert!(!phishing_indicators.is_empty());
}

#[test]
fn test_threat_indicator_creation() {
    let indicator = ThreatIndicator {
        id: "test-indicator".to_string(),
        indicator_type: IndicatorType::IpAddress,
        value: "192.168.1.1".to_string(),
        threat_type: "test".to_string(),
        severity: "high".to_string(),
        sources: vec!["test_source".to_string()],
        first_seen: 1640995200,
        last_seen: 1640995260,
        tags: vec!["test".to_string(), "malware".to_string()],
    };

    assert_eq!(indicator.id, "test-indicator");
    assert_eq!(indicator.value, "192.168.1.1");
    assert_eq!(indicator.threat_type, "test");
    assert_eq!(indicator.severity, "high");
    assert_eq!(indicator.sources.len(), 1);
    assert_eq!(indicator.tags.len(), 2);
}

#[test]
fn test_threat_source_creation() {
    let source = ThreatSource {
        name: "Test Feed".to_string(),
        url: "https://example.com/feed".to_string(),
        last_updated: 1640995200,
        confidence: 0.9,
    };

    assert_eq!(source.name, "Test Feed");
    assert_eq!(source.url, "https://example.com/feed");
    assert_eq!(source.confidence, 0.9);
}

#[test]
fn test_integration_cyber_event_processing() {
    let detector = MaliciousIpDetector::new();
    let pattern_detector = PatternDetector::new();

    // Create suspicious network event
    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "192.168.1.100".to_string(), // Malicious IP
        port: 443,
        protocol: "tcp".to_string(),
        timestamp: 1640995200,
    };

    // Test IP detection
    assert!(detector.is_malicious_ip("192.168.1.100"));

    // Test pattern detection (network patterns)
    let actions = pattern_detector.match_patterns(&[event]);
    // May or may not match depending on patterns, but should not panic
    // Just ensure it returns a Vec (doesn't crash)
    let _actions = actions; // If we reach here, it didn't crash
}

#[test]
fn test_complex_attack_scenario() {
    let ip_detector = MaliciousIpDetector::new();
    let privilege_detector = PrivilegeEscalationDetector::new();
    let pattern_detector = PatternDetector::new();

    // Simulate a complex attack scenario
    let events = vec![
        // Attacker connects from malicious IP
        CyberEvent::NetworkConnection {
            source_ip: "192.168.1.100".to_string(), // Malicious source
            dest_ip: "10.0.0.1".to_string(),
            port: 22,
            protocol: "tcp".to_string(),
            timestamp: 1640995200,
        },
        // Attacker executes dangerous command as admin
        CyberEvent::ProcessExecution {
            process_id: 1234,
            parent_process_id: Some(1),
            command_line: "sudo netcat -l 8080".to_string(),
            user: "admin".to_string(),
            timestamp: 1640995260,
        },
        // Ransomware-like activity
        CyberEvent::ProcessExecution {
            process_id: 1235,
            parent_process_id: Some(1234),
            command_line: "encrypt.exe --target /home --bitcoin abc123".to_string(),
            user: "admin".to_string(),
            timestamp: 1640995320,
        },
    ];

    // Test IP detection
    assert!(ip_detector.is_malicious_ip("192.168.1.100"));

    // Test privilege escalation detection
    assert!(privilege_detector.is_dangerous_command("sudo netcat -l 8080"));

    // Test pattern matching
    let actions = pattern_detector.match_patterns(&events);
    // Should detect ransomware pattern
    assert!(!actions.is_empty());
}
