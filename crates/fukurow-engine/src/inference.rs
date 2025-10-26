//! Inference context management

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Context for inference operations
#[derive(Debug, Clone)]
pub struct InferenceContext {
    /// Variables bound during inference
    variables: HashMap<String, String>,
    /// Timestamp when inference started
    start_time: u64,
    /// Inference metadata
    metadata: HashMap<String, serde_json::Value>,
}

impl InferenceContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        }
    }

    /// Bind a variable to a value
    pub fn bind_variable(&mut self, name: String, value: String) {
        self.variables.insert(name, value);
    }

    /// Get bound variable value
    pub fn get_variable(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }

    /// Check if variable is bound
    pub fn is_bound(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    /// Get all bound variables
    pub fn get_all_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    /// Get inference start time
    pub fn start_time(&self) -> u64 {
        self.start_time
    }

    /// Reset context for new inference
    pub fn reset(&mut self) {
        self.variables.clear();
        self.metadata.clear();
        self.start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Create a child context with inherited variables
    pub fn create_child(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            start_time: self.start_time,
            metadata: self.metadata.clone(),
        }
    }
}

impl Default for InferenceContext {
    fn default() -> Self {
        Self::new()
    }
}
