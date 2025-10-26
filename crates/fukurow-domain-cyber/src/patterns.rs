//! Attack pattern definitions and matching

use fukurow_core::model::{SecurityAction, CyberEvent};
use std::collections::HashMap;

/// Common attack patterns
#[derive(Debug, Clone)]
pub struct AttackPattern {
    pub name: String,
    pub description: String,
    pub indicators: Vec<String>,
    pub severity: String,
    pub actions: Vec<SecurityAction>,
}

/// Pattern-based detector
pub struct PatternDetector {
    patterns: Vec<AttackPattern>,
}

impl PatternDetector {
    pub fn new() -> Self {
        let mut detector = Self {
            patterns: Vec::new(),
        };

        detector.initialize_patterns();
        detector
    }

    fn initialize_patterns(&mut self) {
        // Ransomware pattern
        self.patterns.push(AttackPattern {
            name: "ransomware_execution".to_string(),
            description: "Detect ransomware execution patterns".to_string(),
            indicators: vec![
                "encrypt".to_string(),
                "bitcoin".to_string(),
                "ransom".to_string(),
                ".encrypted".to_string(),
            ],
            severity: "critical".to_string(),
            actions: vec![
                SecurityAction::IsolateHost {
                    host_ip: "?host_ip".to_string(),
                    reason: "Ransomware execution detected".to_string(),
                },
                SecurityAction::TerminateProcess {
                    process_id: 0, // Will be filled by detection logic
                    reason: "Ransomware process termination".to_string(),
                },
                SecurityAction::Alert {
                    severity: "critical".to_string(),
                    message: "Ransomware attack detected".to_string(),
                    details: serde_json::json!({
                        "pattern": "ransomware_execution",
                        "indicators": ["encrypt", "bitcoin", "ransom"]
                    }),
                },
            ],
        });

        // Data exfiltration pattern
        self.patterns.push(AttackPattern {
            name: "data_exfiltration".to_string(),
            description: "Detect large data transfers to external IPs".to_string(),
            indicators: vec![
                "large_file_transfer".to_string(),
                "external_ip".to_string(),
                "unusual_traffic".to_string(),
            ],
            severity: "high".to_string(),
            actions: vec![
                SecurityAction::BlockConnection {
                    source_ip: "?source_ip".to_string(),
                    dest_ip: "?dest_ip".to_string(),
                    reason: "Potential data exfiltration detected".to_string(),
                },
                SecurityAction::Alert {
                    severity: "high".to_string(),
                    message: "Data exfiltration attempt".to_string(),
                    details: serde_json::json!({
                        "pattern": "data_exfiltration",
                        "traffic_type": "large_transfer"
                    }),
                },
            ],
        });

        // Brute force attack pattern
        self.patterns.push(AttackPattern {
            name: "brute_force_login".to_string(),
            description: "Detect brute force login attempts".to_string(),
            indicators: vec![
                "multiple_failed_logins".to_string(),
                "same_user".to_string(),
                "rapid_attempts".to_string(),
            ],
            severity: "medium".to_string(),
            actions: vec![
                SecurityAction::BlockConnection {
                    source_ip: "?source_ip".to_string(),
                    dest_ip: "?dest_ip".to_string(),
                    reason: "Brute force login attempts detected".to_string(),
                },
                SecurityAction::Alert {
                    severity: "medium".to_string(),
                    message: "Brute force attack detected".to_string(),
                    details: serde_json::json!({
                        "pattern": "brute_force_login",
                        "attempt_count": "?count"
                    }),
                },
            ],
        });
    }

    /// Match events against attack patterns
    pub fn match_patterns(&self, events: &[CyberEvent]) -> Vec<SecurityAction> {
        let mut actions = Vec::new();

        for pattern in &self.patterns {
            if self.pattern_matches_events(pattern, events) {
                actions.extend(pattern.actions.clone());
            }
        }

        actions
    }

    fn pattern_matches_events(&self, pattern: &AttackPattern, events: &[CyberEvent]) -> bool {
        // Simple pattern matching - in real implementation, this would be more sophisticated
        for event in events {
            match event {
                CyberEvent::ProcessExecution { command_line, .. } => {
                    if pattern.indicators.iter().any(|indicator| command_line.contains(indicator)) {
                        return true;
                    }
                }
                CyberEvent::NetworkConnection { .. } => {
                    // Network pattern matching would go here
                }
                CyberEvent::FileAccess { .. } => {
                    // File access pattern matching would go here
                }
                CyberEvent::UserLogin { .. } => {
                    // Login pattern matching would go here
                }
            }
        }
        false
    }

    /// Get all available patterns
    pub fn get_patterns(&self) -> &[AttackPattern] {
        &self.patterns
    }
}

/// Behavioral anomaly detector
pub struct AnomalyDetector {
    baseline_metrics: HashMap<String, f64>,
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            baseline_metrics: HashMap::new(),
        }
    }

    /// Update baseline metrics
    pub fn update_baseline(&mut self, metric_name: String, value: f64) {
        self.baseline_metrics.insert(metric_name, value);
    }

    /// Detect anomalies in metrics
    pub fn detect_anomaly(&self, metric_name: &str, current_value: f64, threshold: f64) -> Option<SecurityAction> {
        if let Some(baseline) = self.baseline_metrics.get(metric_name) {
            let deviation = (current_value - baseline).abs() / baseline;

            if deviation > threshold {
                return Some(SecurityAction::Alert {
                    severity: "medium".to_string(),
                    message: format!("Anomaly detected in {}", metric_name),
                    details: serde_json::json!({
                        "metric": metric_name,
                        "baseline": baseline,
                        "current": current_value,
                        "deviation_percent": deviation * 100.0
                    }),
                });
            }
        }
        None
    }
}
