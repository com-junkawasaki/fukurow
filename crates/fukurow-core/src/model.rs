//! Graph data models for JSON-LD reasoning

use serde::{Deserialize, Serialize};
// Sophia API imports removed - using simple string-based representation
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use lazy_static::lazy_static;

/// Global string interning pool for memory optimization
lazy_static! {
    static ref STRING_POOL: Arc<RwLock<HashMap<String, Arc<String>>>> = Arc::new(RwLock::new(HashMap::new()));
}

/// Interned string that reuses memory for identical strings
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternedString(Arc<String>);

impl InternedString {
    /// Create a new interned string, reusing existing instances when possible
    pub fn new<S: Into<String>>(s: S) -> Self {
        let string = s.into();
        let mut pool = STRING_POOL.write().unwrap();

        if let Some(interned) = pool.get(&string) {
            InternedString(Arc::clone(interned))
        } else {
            let interned = Arc::new(string.clone());
            pool.insert(string, Arc::clone(&interned));
            InternedString(interned)
        }
    }

    /// Get the string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the length
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for InternedString {
    fn from(s: String) -> Self {
        InternedString::new(s)
    }
}

impl From<&str> for InternedString {
    fn from(s: &str) -> Self {
        InternedString::new(s)
    }
}

impl From<&String> for InternedString {
    fn from(s: &String) -> Self {
        InternedString::new(s.clone())
    }
}

impl std::fmt::Display for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for InternedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// RDF Triple representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Triple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// Memory-optimized RDF Triple using interned strings
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternedTriple {
    pub subject: InternedString,
    pub predicate: InternedString,
    pub object: InternedString,
}

impl InternedTriple {
    /// Create a new interned triple
    pub fn new<S: Into<InternedString>, P: Into<InternedString>, O: Into<InternedString>>(
        subject: S,
        predicate: P,
        object: O,
    ) -> Self {
        InternedTriple {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
        }
    }

    /// Convert to regular Triple
    pub fn to_triple(&self) -> Triple {
        Triple {
            subject: self.subject.as_str().to_string(),
            predicate: self.predicate.as_str().to_string(),
            object: self.object.as_str().to_string(),
        }
    }

    /// Convert from regular Triple
    pub fn from_triple(triple: &Triple) -> Self {
        InternedTriple::new(
            InternedString::from(&triple.subject),
            InternedString::from(&triple.predicate),
            InternedString::from(&triple.object),
        )
    }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRule {
    pub name: String,
    pub description: String,
    pub conditions: Vec<Triple>,
    pub actions: Vec<SecurityAction>,
}
