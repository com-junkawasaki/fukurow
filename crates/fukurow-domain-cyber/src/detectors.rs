//! Cyber security threat detectors

use fukurow_core::model::{SecurityAction, CyberEvent, InferenceRule, Triple};
use regex::Regex;
use std::collections::HashSet;

/// Malicious IP detector
pub struct MaliciousIpDetector {
    known_malicious_ips: HashSet<String>,
    suspicious_patterns: Vec<Regex>,
}

impl MaliciousIpDetector {
    pub fn new() -> Self {
        let mut detector = Self {
            known_malicious_ips: HashSet::new(),
            suspicious_patterns: Vec::new(),
        };

        // Initialize with known malicious IPs (in real implementation, this would come from threat intelligence feeds)
        detector.known_malicious_ips.insert("192.168.1.100".to_string());
        detector.known_malicious_ips.insert("10.0.0.50".to_string());

        // Suspicious IP patterns
        detector.suspicious_patterns.push(Regex::new(r"^192\.168\.1\.\d+$").unwrap());
        detector.suspicious_patterns.push(Regex::new(r"^10\.0\.0\.\d+$").unwrap());

        detector
    }

    pub fn is_malicious_ip(&self, ip: &str) -> bool {
        self.known_malicious_ips.contains(ip) ||
        self.suspicious_patterns.iter().any(|pattern| pattern.is_match(ip))
    }

    pub fn create_rule(&self) -> InferenceRule {
        InferenceRule {
            name: "advanced_malicious_ip_detection".to_string(),
            description: "Advanced malicious IP detection using patterns and threat intelligence".to_string(),
            conditions: vec![
                Triple {
                    subject: "?connection".to_string(),
                    predicate: "rdf:type".to_string(),
                    object: "https://w3id.org/security#NetworkConnection".to_string(),
                },
                Triple {
                    subject: "?connection".to_string(),
                    predicate: "https://w3id.org/security#destIp".to_string(),
                    object: "?dest_ip".to_string(),
                },
            ],
            actions: vec![
                SecurityAction::BlockConnection {
                    source_ip: "?source_ip".to_string(),
                    dest_ip: "?dest_ip".to_string(),
                    reason: "Connection to malicious IP detected by advanced pattern matching".to_string(),
                },
                SecurityAction::Alert {
                    severity: "high".to_string(),
                    message: "Malicious IP connection blocked".to_string(),
                    details: serde_json::json!({
                        "connection_id": "?connection",
                        "destination_ip": "?dest_ip",
                        "detection_method": "pattern_matching"
                    }),
                },
            ],
        }
    }
}

/// Lateral movement detector
pub struct LateralMovementDetector {
    time_window_seconds: u64,
}

impl LateralMovementDetector {
    pub fn new() -> Self {
        Self {
            time_window_seconds: 300, // 5 minutes
        }
    }

    pub fn detect_lateral_movement(&self, events: &[CyberEvent]) -> Vec<SecurityAction> {
        let mut actions = Vec::new();
        let mut user_sessions: std::collections::HashMap<String, Vec<(String, i64)>> = std::collections::HashMap::new();

        // Group login events by user
        for event in events {
            if let CyberEvent::UserLogin { user, source_ip, success, timestamp } = event {
                if *success {
                    user_sessions.entry(user.clone())
                        .or_insert_with(Vec::new)
                        .push((source_ip.clone(), *timestamp));
                }
            }
        }

        // Check for lateral movement patterns
        for (user, mut sessions) in user_sessions {
            if sessions.len() >= 3 {
                sessions.sort_by_key(|(_, ts)| *ts);

                // Check for rapid movement between different IPs
                for window in sessions.windows(3) {
                    let time_span = window[2].1 - window[0].1;
                    let unique_ips: HashSet<_> = window.iter().map(|(ip, _)| ip).collect();

                    if time_span <= self.time_window_seconds as i64 && unique_ips.len() >= 3 {
                        actions.push(SecurityAction::Alert {
                            severity: "high".to_string(),
                            message: "Rapid lateral movement detected".to_string(),
                            details: serde_json::json!({
                                "user": user,
                                "session_count": sessions.len(),
                                "time_span_seconds": time_span,
                                "unique_ips": unique_ips.len()
                            }),
                        });
                        break;
                    }
                }
            }
        }

        actions
    }

    pub fn create_rule(&self) -> InferenceRule {
        InferenceRule {
            name: "lateral_movement_detection".to_string(),
            description: "Detect lateral movement patterns across multiple hosts".to_string(),
            conditions: vec![
                Triple {
                    subject: "?user".to_string(),
                    predicate: "https://w3id.org/security#hasSession".to_string(),
                    object: "?session1".to_string(),
                },
                Triple {
                    subject: "?user".to_string(),
                    predicate: "https://w3id.org/security#hasSession".to_string(),
                    object: "?session2".to_string(),
                },
                Triple {
                    subject: "?session1".to_string(),
                    predicate: "https://w3id.org/security#sourceIp".to_string(),
                    object: "?ip1".to_string(),
                },
                Triple {
                    subject: "?session2".to_string(),
                    predicate: "https://w3id.org/security#sourceIp".to_string(),
                    object: "?ip2".to_string(),
                },
            ],
            actions: vec![
                SecurityAction::Alert {
                    severity: "medium".to_string(),
                    message: "Potential lateral movement detected".to_string(),
                    details: serde_json::json!({
                        "user": "?user",
                        "from_ip": "?ip1",
                        "to_ip": "?ip2"
                    }),
                },
            ],
        }
    }
}

/// Privilege escalation detector
pub struct PrivilegeEscalationDetector {
    dangerous_commands: HashSet<String>,
}

impl PrivilegeEscalationDetector {
    pub fn new() -> Self {
        let mut detector = Self {
            dangerous_commands: HashSet::new(),
        };

        // Initialize dangerous command patterns
        detector.dangerous_commands.insert("sudo".to_string());
        detector.dangerous_commands.insert("su".to_string());
        detector.dangerous_commands.insert("chmod 777".to_string());
        detector.dangerous_commands.insert("rm -rf /".to_string());
        detector.dangerous_commands.insert("netcat".to_string());
        detector.dangerous_commands.insert("nmap".to_string());

        detector
    }

    pub fn is_dangerous_command(&self, command: &str) -> bool {
        self.dangerous_commands.iter().any(|dangerous| command.contains(dangerous))
    }

    pub fn create_rule(&self) -> InferenceRule {
        InferenceRule {
            name: "privilege_escalation_detection".to_string(),
            description: "Detect privilege escalation through dangerous command execution".to_string(),
            conditions: vec![
                Triple {
                    subject: "?process".to_string(),
                    predicate: "rdf:type".to_string(),
                    object: "https://w3id.org/security#ProcessExecution".to_string(),
                },
                Triple {
                    subject: "?process".to_string(),
                    predicate: "https://w3id.org/security#user".to_string(),
                    object: "?user".to_string(),
                },
                Triple {
                    subject: "?user".to_string(),
                    predicate: "https://w3id.org/security#privilegeLevel".to_string(),
                    object: "admin".to_string(),
                },
                Triple {
                    subject: "?process".to_string(),
                    predicate: "https://w3id.org/security#commandLine".to_string(),
                    object: "?command".to_string(),
                },
            ],
            actions: vec![
                SecurityAction::IsolateHost {
                    host_ip: "?host_ip".to_string(),
                    reason: "Privileged user executing dangerous commands".to_string(),
                },
                SecurityAction::RevokePrivileges {
                    user: "?user".to_string(),
                    privilege: "admin".to_string(),
                    reason: "Dangerous command execution detected".to_string(),
                },
                SecurityAction::Alert {
                    severity: "critical".to_string(),
                    message: "Privilege escalation alert".to_string(),
                    details: serde_json::json!({
                        "user": "?user",
                        "process": "?process",
                        "command": "?command"
                    }),
                },
            ],
        }
    }
}
