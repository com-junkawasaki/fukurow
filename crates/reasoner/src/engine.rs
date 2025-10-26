//! Inference engine for security event reasoning

use crate::rules::RuleEngine;
use crate::inference::InferenceContext;
use reasoner_graph::model::{CyberEvent, SecurityAction, InferenceRule};
use reasoner_graph::store::GraphStore;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Main reasoning engine
pub struct ReasonerEngine {
    graph_store: Arc<RwLock<GraphStore>>,
    rule_engine: RuleEngine,
    context: InferenceContext,
}

impl ReasonerEngine {
    /// Create new reasoning engine
    pub fn new() -> Self {
        Self {
            graph_store: Arc::new(RwLock::new(GraphStore::new())),
            rule_engine: RuleEngine::new(),
            context: InferenceContext::new(),
        }
    }

    /// Add a cyber security event for reasoning
    pub async fn add_event(&self, event: CyberEvent) -> Result<(), ReasonerError> {
        info!("Adding cyber event: {:?}", event);

        // Convert event to JSON-LD and add to graph
        let jsonld_doc = reasoner_graph::jsonld::cyber_event_to_jsonld(&event)
            .map_err(ReasonerError::GraphError)?;

        let triples = reasoner_graph::jsonld::jsonld_to_triples(&jsonld_doc)
            .map_err(ReasonerError::GraphError)?;

        let mut store = self.graph_store.write().await;
        store.add_triples_to_graph("events", triples);

        Ok(())
    }

    /// Execute reasoning and return proposed security actions
    /// No side effects - only returns action proposals
    pub async fn reason(&self) -> Result<Vec<SecurityAction>, ReasonerError> {
        info!("Starting reasoning process");

        let store = self.graph_store.read().await;
        let mut actions = Vec::new();

        // Apply all rules
        for rule in self.rule_engine.get_rules() {
            match self.rule_engine.evaluate_rule(&store, rule, &self.context).await {
                Ok(rule_actions) => {
                    actions.extend(rule_actions);
                }
                Err(e) => {
                    warn!("Rule evaluation failed for {}: {:?}", rule.name, e);
                }
            }
        }

        // Remove duplicates while preserving order
        let mut unique_actions = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for action in actions {
            let key = format!("{:?}", action);
            if seen.insert(key) {
                unique_actions.push(action);
            }
        }

        info!("Reasoning complete, proposed {} actions", unique_actions.len());
        Ok(unique_actions)
    }

    /// Get current graph store (read-only access)
    pub async fn get_graph_store(&self) -> Arc<RwLock<GraphStore>> {
        Arc::clone(&self.graph_store)
    }

    /// Clear all events and reset reasoning state
    pub async fn reset(&mut self) -> Result<(), ReasonerError> {
        let mut store = self.graph_store.write().await;
        store.clear();
        self.context.reset();
        Ok(())
    }

    /// Add custom inference rule
    pub fn add_rule(&mut self, rule: InferenceRule) {
        self.rule_engine.add_rule(rule);
    }
}

/// Reasoning engine errors
#[derive(Debug, thiserror::Error)]
pub enum ReasonerError {
    #[error("Graph operation error: {0}")]
    GraphError(#[from] anyhow::Error),

    #[error("Rule evaluation error: {0}")]
    RuleError(String),

    #[error("Inference context error: {0}")]
    ContextError(String),
}
