//! Execution context for reasoning operations

use crate::inference::InferenceContext;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe reasoning context
#[derive(Clone)]
pub struct ReasoningContext {
    inner: Arc<RwLock<InferenceContext>>,
}

impl ReasoningContext {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(InferenceContext::new())),
        }
    }

    /// Bind a variable (async)
    pub async fn bind_variable(&self, name: String, value: String) {
        let mut ctx = self.inner.write().await;
        ctx.bind_variable(name, value);
    }

    /// Get variable value (async)
    pub async fn get_variable(&self, name: &str) -> Option<String> {
        let ctx = self.inner.read().await;
        ctx.get_variable(name).cloned()
    }

    /// Set metadata (async)
    pub async fn set_metadata(&self, key: String, value: serde_json::Value) {
        let mut ctx = self.inner.write().await;
        ctx.set_metadata(key, value);
    }

    /// Get metadata (async)
    pub async fn get_metadata(&self, key: &str) -> Option<serde_json::Value> {
        let ctx = self.inner.read().await;
        ctx.get_metadata(key).cloned()
    }

    /// Reset context
    pub async fn reset(&self) {
        let mut ctx = self.inner.write().await;
        ctx.reset();
    }

    /// Get start time
    pub async fn start_time(&self) -> u64 {
        let ctx = self.inner.read().await;
        ctx.start_time()
    }
}

impl Default for ReasoningContext {
    fn default() -> Self {
        Self::new()
    }
}
