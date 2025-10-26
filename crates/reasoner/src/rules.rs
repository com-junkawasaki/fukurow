//! Rule evaluation engine

use reasoner_graph::model::{InferenceRule, SecurityAction, Triple};
use reasoner_graph::store::GraphStore;
use reasoner_graph::query::{GraphQuery, var, const_val};
use crate::inference::InferenceContext;
use std::collections::HashMap;
use anyhow::Result;

/// Rule evaluation engine
#[derive(Debug)]
pub struct RuleEngine {
    rules: Vec<InferenceRule>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    /// Add a new inference rule
    pub fn add_rule(&mut self, rule: InferenceRule) {
        self.rules.push(rule);
    }

    /// Get all rules
    pub fn get_rules(&self) -> &[InferenceRule] {
        &self.rules
    }

    /// Evaluate a single rule against the current graph state
    pub async fn evaluate_rule(
        &self,
        store: &GraphStore,
        rule: &InferenceRule,
        context: &InferenceContext,
    ) -> Result<Vec<SecurityAction>> {
        // Check if rule conditions are met
        if self.check_conditions(store, &rule.conditions) {
            // Rule fired - return actions
            Ok(rule.actions.clone())
        } else {
            Ok(Vec::new())
        }
    }

    /// Check if rule conditions are satisfied
    fn check_conditions(&self, store: &GraphStore, conditions: &[Triple]) -> bool {
        for condition in conditions {
            let matches = store.find_triples(
                Some(&condition.subject),
                Some(&condition.predicate),
                Some(&condition.object),
            );

            if matches.is_empty() {
                return false;
            }
        }
        true
    }

    /// Add built-in cyber security rules
    pub fn add_default_cyber_rules(&mut self) {
        // Rule 1: Malicious IP connection detection
        self.add_rule(InferenceRule {
            name: "malicious_ip_connection".to_string(),
            description: "Detect connections to known malicious IPs".to_string(),
            conditions: vec![
                Triple {
                    subject: "?connection".to_string(),
                    predicate: "type".to_string(),
                    object: "NetworkConnection".to_string(),
                },
                Triple {
                    subject: "?connection".to_string(),
                    predicate: "destIp".to_string(),
                    object: "?malicious_ip".to_string(),
                },
                Triple {
                    subject: "?malicious_ip".to_string(),
                    predicate: "threatLevel".to_string(),
                    object: "high".to_string(),
                },
            ],
            actions: vec![
                SecurityAction::BlockConnection {
                    source_ip: "?source_ip".to_string(),
                    dest_ip: "?malicious_ip".to_string(),
                    reason: "Connection to known malicious IP detected".to_string(),
                },
                SecurityAction::Alert {
                    severity: "high".to_string(),
                    message: "Malicious IP connection attempt".to_string(),
                    details: serde_json::json!({
                        "connection": "?connection",
                        "malicious_ip": "?malicious_ip"
                    }),
                },
            ],
        });

        // Rule 2: Lateral movement detection
        self.add_rule(InferenceRule {
            name: "lateral_movement".to_string(),
            description: "Detect potential lateral movement between hosts".to_string(),
            conditions: vec![
                Triple {
                    subject: "?login1".to_string(),
                    predicate: "type".to_string(),
                    object: "UserLogin".to_string(),
                },
                Triple {
                    subject: "?login1".to_string(),
                    predicate: "user".to_string(),
                    object: "?user".to_string(),
                },
                Triple {
                    subject: "?login1".to_string(),
                    predicate: "sourceIp".to_string(),
                    object: "?ip1".to_string(),
                },
                Triple {
                    subject: "?login2".to_string(),
                    predicate: "type".to_string(),
                    object: "UserLogin".to_string(),
                },
                Triple {
                    subject: "?login2".to_string(),
                    predicate: "user".to_string(),
                    object: "?user".to_string(),
                },
                Triple {
                    subject: "?login2".to_string(),
                    predicate: "sourceIp".to_string(),
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
        });

        // Rule 3: Privilege escalation detection
        self.add_rule(InferenceRule {
            name: "privilege_escalation".to_string(),
            description: "Detect dangerous use of privileged accounts".to_string(),
            conditions: vec![
                Triple {
                    subject: "?process".to_string(),
                    predicate: "type".to_string(),
                    object: "ProcessExecution".to_string(),
                },
                Triple {
                    subject: "?process".to_string(),
                    predicate: "user".to_string(),
                    object: "?privileged_user".to_string(),
                },
                Triple {
                    subject: "?privileged_user".to_string(),
                    predicate: "privilegeLevel".to_string(),
                    object: "admin".to_string(),
                },
                Triple {
                    subject: "?process".to_string(),
                    predicate: "commandLine".to_string(),
                    object: "?suspicious_cmd".to_string(),
                },
            ],
            actions: vec![
                SecurityAction::IsolateHost {
                    host_ip: "?host_ip".to_string(),
                    reason: "Privileged account executing suspicious commands".to_string(),
                },
                SecurityAction::Alert {
                    severity: "critical".to_string(),
                    message: "Privilege escalation detected".to_string(),
                    details: serde_json::json!({
                        "user": "?privileged_user",
                        "process": "?process",
                        "command": "?suspicious_cmd"
                    }),
                },
            ],
        });
    }
}
