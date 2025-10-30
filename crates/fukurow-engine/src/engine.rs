//! Inference engine for security event reasoning

use fukurow_core::model::{CyberEvent, SecurityAction, InferenceRule};
use fukurow_store::{store::RdfStore, Triple};
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
    /// Convert a CyberEvent to a vector of Triples
    fn cyber_event_to_triples(event: &CyberEvent) -> Vec<fukurow_store::Triple> {
        let mut triples = Vec::new();
        let (subject, timestamp) = match event {
            CyberEvent::NetworkConnection { timestamp, .. } => (format!("event:{}", timestamp), *timestamp),
            CyberEvent::ProcessExecution { timestamp, .. } => (format!("event:{}", timestamp), *timestamp),
            CyberEvent::FileAccess { timestamp, .. } => (format!("event:{}", timestamp), *timestamp),
            CyberEvent::UserLogin { timestamp, .. } => (format!("event:{}", timestamp), *timestamp),
        };

        // Add type triple
        triples.push(fukurow_store::Triple {
            subject: subject.clone(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            object: "http://example.org/CyberEvent".to_string(),
        });

        // Add event-specific triples based on variant
        match event {
            CyberEvent::NetworkConnection { source_ip, dest_ip, port, protocol, timestamp } => {
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/sourceIP".to_string(),
                    object: source_ip.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/destIP".to_string(),
                    object: dest_ip.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/port".to_string(),
                    object: port.to_string(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/protocol".to_string(),
                    object: protocol.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/timestamp".to_string(),
                    object: timestamp.to_string(),
                });
            }
            CyberEvent::ProcessExecution { process_id, parent_process_id, command_line, user, timestamp } => {
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/processId".to_string(),
                    object: process_id.to_string(),
                });
                if let Some(parent_id) = parent_process_id {
                    triples.push(fukurow_store::Triple {
                        subject: subject.clone(),
                        predicate: "http://example.org/parentProcessId".to_string(),
                        object: parent_id.to_string(),
                    });
                }
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/commandLine".to_string(),
                    object: command_line.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/user".to_string(),
                    object: user.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/timestamp".to_string(),
                    object: timestamp.to_string(),
                });
            }
            CyberEvent::FileAccess { file_path, access_type, user, process_id, timestamp } => {
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/filePath".to_string(),
                    object: file_path.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/accessType".to_string(),
                    object: access_type.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/user".to_string(),
                    object: user.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/processId".to_string(),
                    object: process_id.to_string(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/timestamp".to_string(),
                    object: timestamp.to_string(),
                });
            }
            CyberEvent::UserLogin { user, source_ip, success, timestamp } => {
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/user".to_string(),
                    object: user.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/sourceIP".to_string(),
                    object: source_ip.clone(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/success".to_string(),
                    object: success.to_string(),
                });
                triples.push(fukurow_store::Triple {
                    subject: subject.clone(),
                    predicate: "http://example.org/timestamp".to_string(),
                    object: timestamp.to_string(),
                });
            }
        }

        triples
    }
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

        // Convert event to triples directly
        let triples = Self::cyber_event_to_triples(&event);

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

    /// Process an external RDF store and return reasoning results
    pub async fn process(&self, store: &RdfStore) -> Result<super::orchestration::EngineResult, ReasonerError> {
        self.reasoning_engine.process(store).await
            .map_err(|e| ReasonerError::ReasoningError(e.to_string()))
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
