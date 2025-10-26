//! Graph data models for JSON-LD reasoning

use serde::{Deserialize, Serialize};
// Sophia API imports removed - using simple string-based representation
use std::collections::HashMap;

/// RDF Triple representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Triple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// JSON-LD Document with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonLdDocument {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    #[serde(rename = "@graph", skip_serializing_if = "Option::is_none")]
    pub graph: Option<Vec<serde_json::Value>>,
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

/// Named Graph for organizing triples
#[derive(Debug, Clone, Default)]
pub struct NamedGraph {
    pub name: String,
    pub triples: Vec<Triple>,
}

/// Event types in cyber security domain
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CyberEvent {
    NetworkConnection {
        source_ip: String,
        dest_ip: String,
        port: u16,
        protocol: String,
        timestamp: i64,
    },
    ProcessExecution {
        process_id: u32,
        parent_process_id: Option<u32>,
        command_line: String,
        user: String,
        timestamp: i64,
    },
    FileAccess {
        file_path: String,
        access_type: String,
        user: String,
        process_id: u32,
        timestamp: i64,
    },
    UserLogin {
        user: String,
        source_ip: String,
        success: bool,
        timestamp: i64,
    },
}

/// Security actions that can be proposed by the reasoner
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action_type", content = "parameters")]
pub enum SecurityAction {
    IsolateHost { host_ip: String, reason: String },
    BlockConnection { source_ip: String, dest_ip: String, reason: String },
    TerminateProcess { process_id: u32, reason: String },
    RevokePrivileges { user: String, privilege: String, reason: String },
    Alert { severity: String, message: String, details: serde_json::Value },
}

/// Inference rule for pattern matching
#[derive(Debug, Clone)]
pub struct InferenceRule {
    pub name: String,
    pub description: String,
    pub conditions: Vec<Triple>,
    pub actions: Vec<SecurityAction>,
}
