//! Provenance and audit trail management

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Provenance information for stored triples
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Provenance {
    /// Data observed from sensors/agents
    Sensor {
        /// Source identifier (e.g., "edr-agent-001", "firewall-192.168.1.1")
        source: String,
        /// Confidence score (0.0 to 1.0)
        confidence: Option<f64>,
    },
    /// Data inferred by reasoning engine
    Inferred {
        /// Rule that performed the inference
        rule: String,
        /// Reasoning level (rdfs, owl-lite, owl-dl, etc.)
        reasoning_level: String,
        /// Supporting evidence (other triples that led to this inference)
        evidence: Vec<String>,
    },
    /// Data loaded from external sources
    Imported {
        /// Import source (URI, file path, etc.)
        source_uri: String,
        /// Import timestamp
        imported_at: DateTime<Utc>,
    },
}

/// Graph identifier for organizing triples
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GraphId {
    /// Default graph
    Default,
    /// Named graph
    Named(String),
    /// Sensor-specific graph
    Sensor(String),
    /// Inferred triples graph
    Inferred(String),
}

/// Audit trail entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique audit ID
    pub id: String,
    /// Timestamp of the operation
    pub timestamp: DateTime<Utc>,
    /// Operation type
    pub operation: AuditOperation,
    /// User/context that performed the operation
    pub actor: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of audit operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditOperation {
    /// Triple inserted
    Insert {
        triple: String,
        graph_id: GraphId,
        provenance: Provenance,
    },
    /// Triple deleted
    Delete {
        triple: String,
        graph_id: GraphId,
    },
    /// Graph cleared
    Clear {
        graph_id: GraphId,
        triple_count: usize,
    },
    /// Inference executed
    Inference {
        rule: String,
        triples_added: usize,
        triples_removed: usize,
    },
    /// Query executed
    Query {
        query_type: String,
        result_count: usize,
    },
}

impl Default for GraphId {
    fn default() -> Self {
        GraphId::Default
    }
}

impl std::fmt::Display for GraphId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphId::Default => write!(f, "default"),
            GraphId::Named(name) => write!(f, "named:{}", name),
            GraphId::Sensor(sensor) => write!(f, "sensor:{}", sensor),
            GraphId::Inferred(rule) => write!(f, "inferred:{}", rule),
        }
    }
}
