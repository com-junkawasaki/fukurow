//! Inference engine for security event reasoning

use fukurow_core::model::{CyberEvent, SecurityAction, InferenceRule};
use fukurow_store::store::RdfStore;
use fukurow_rules::{RuleRegistry, Rule};
use super::orchestration::{ReasoningEngine, ProcessingOptions};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Compatibility layer for legacy ReasonerEngine API
/// Delegates to the new ReasoningEngine
pub struct ReasonerEngine {
    rdf_store: Arc<RwLock<RdfStore>>,
    reasoning_engine: ReasoningEngine,
}

impl ReasonerEngine {
    /// Create new reasoning engine
    pub fn new() -> Self {
        let rdf_store = Arc::new(RwLock::new(RdfStore::new()));
        let reasoning_engine = ReasoningEngine::new();

        Self {
            rdf_store,
            reasoning_engine,
        }
    }

    /// Add a cyber security event for reasoning
    pub async fn add_event(&self, event: CyberEvent) -> Result<(), ReasonerError> {
        info!("Adding cyber event: {:?}", event);

        // Convert event to JSON-LD and add to graph
        let jsonld_doc = fukurow_core::jsonld::cyber_event_to_jsonld(&event)
            .map_err(ReasonerError::GraphError)?;

        let triples = fukurow_core::jsonld::jsonld_to_triples(&jsonld_doc)
            .map_err(ReasonerError::GraphError)?;

        let mut store = self.rdf_store.write().await;
        for triple in triples {
            store.insert(triple, fukurow_store::provenance::GraphId::Named("events".to_string()),
                         fukurow_store::provenance::Provenance::Sensor {
                             source: "reasoner-engine".to_string(),
                             confidence: None,
                         });
        }

        Ok(())
    }

    /// Execute reasoning and return proposed security actions
    /// No side effects - only returns action proposals
    pub async fn reason(&self) -> Result<Vec<SecurityAction>, ReasonerError> {
        info!("Starting reasoning process");

        let store = self.rdf_store.read().await;
        let result = self.reasoning_engine.process(&store).await
            .map_err(|e| ReasonerError::ReasoningError(e.to_string()))?;

        info!("Reasoning complete, proposed {} actions", result.actions.len());
        Ok(result.actions)
    }

    /// Get current graph store (read-only access)
    pub async fn get_graph_store(&self) -> Arc<RwLock<RdfStore>> {
        Arc::clone(&self.rdf_store)
    }

    /// Clear all events and reset reasoning state
    pub async fn reset(&mut self) -> Result<(), ReasonerError> {
        let mut store = self.rdf_store.write().await;
        store.clear_all();
        Ok(())
    }

    /// Add custom inference rule
    pub fn add_rule(&mut self, _rule: InferenceRule) {
        // TODO: Implement rule addition for new architecture
        warn!("Rule addition not yet implemented in new architecture");
    }
}

/// Reasoning engine errors
#[derive(Debug, thiserror::Error)]
pub enum ReasonerError {
    #[error("Graph operation error: {0}")]
    GraphError(#[from] anyhow::Error),

    #[error("Rule evaluation error: {0}")]
    RuleError(String),

    #[error("Reasoning process error: {0}")]
    ReasoningError(String),

    #[error("Store operation error: {0}")]
    StoreError(String),
}
