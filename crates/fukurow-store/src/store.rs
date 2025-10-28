//! RDF Store implementation with provenance

use fukurow_core::model::Triple;
use crate::provenance::{Provenance, GraphId, AuditEntry, AuditOperation};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Stored triple with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTriple {
    /// Graph identifier
    pub graph_id: GraphId,
    /// The RDF triple
    pub triple: Triple,
    /// When this triple was asserted (Unix timestamp in milliseconds)
    pub asserted_at: u64,
    /// Provenance information
    pub provenance: Provenance,
}

/// RDF Store with provenance tracking
#[derive(Debug)]
pub struct RdfStore {
    /// All stored triples, indexed by graph
    triples: HashMap<GraphId, Vec<StoredTriple>>,
    /// Audit trail (limited size for memory efficiency)
    audit_trail: Vec<AuditEntry>,
    /// Subject index for fast lookup
    subject_index: HashMap<String, HashSet<(GraphId, usize)>>,
    /// Predicate index for fast lookup
    predicate_index: HashMap<String, HashSet<(GraphId, usize)>>,
    /// Object index for fast lookup
    object_index: HashMap<String, HashSet<(GraphId, usize)>>,
    /// Maximum audit trail size (for memory management)
    max_audit_entries: usize,
}

impl RdfStore {
    /// Create a new empty RDF store
    pub fn new() -> Self {
        Self::with_audit_limit(1000) // Default audit trail limit
    }

    /// Create a new RDF store with custom audit trail limit
    pub fn with_audit_limit(max_audit_entries: usize) -> Self {
        Self {
            triples: HashMap::new(),
            audit_trail: Vec::new(),
            subject_index: HashMap::new(),
            predicate_index: HashMap::new(),
            object_index: HashMap::new(),
            max_audit_entries,
        }
    }

    /// Insert a triple with provenance
    pub fn insert(&mut self, triple: Triple, graph_id: GraphId, provenance: Provenance) {
        let stored = StoredTriple {
            graph_id: graph_id.clone(),
            triple: triple.clone(),
            asserted_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            provenance: provenance.clone(),
        };

        let graph = self.triples.entry(graph_id.clone()).or_insert_with(Vec::new);
        let index = graph.len();
        graph.push(stored);

        // Update indices
        self.subject_index.entry(triple.subject.clone())
            .or_insert_with(HashSet::new)
            .insert((graph_id.clone(), index));
        self.predicate_index.entry(triple.predicate.clone())
            .or_insert_with(HashSet::new)
            .insert((graph_id.clone(), index));
        self.object_index.entry(triple.object.clone())
            .or_insert_with(HashSet::new)
            .insert((graph_id.clone(), index));

        // Audit trail with memory management
        self.add_audit_entry(AuditEntry {
            id: format!("audit-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            operation: AuditOperation::Insert {
                triple: format!("{} {} {}", triple.subject, triple.predicate, triple.object),
                graph_id,
                provenance,
            },
            actor: None,
            metadata: HashMap::new(),
        });
    }

    /// Insert multiple triples with the same provenance
    pub fn insert_batch(&mut self, triples: Vec<Triple>, graph_id: GraphId, provenance: Provenance) {
        for triple in triples {
            self.insert(triple, graph_id.clone(), provenance.clone());
        }
    }

    /// Find triples matching a pattern
    pub fn find_triples(&self, subject: Option<&str>, predicate: Option<&str>, object: Option<&str>) -> Vec<&StoredTriple> {
        let mut candidates = Vec::new();

        // Use the most selective index
        if let Some(subj) = subject {
            if let Some(indices) = self.subject_index.get(subj) {
                for (graph_id, idx) in indices {
                    if let Some(graph) = self.triples.get(graph_id) {
                        if let Some(stored) = graph.get(*idx) {
                            candidates.push(stored);
                        }
                    }
                }
            }
        } else if let Some(pred) = predicate {
            if let Some(indices) = self.predicate_index.get(pred) {
                for (graph_id, idx) in indices {
                    if let Some(graph) = self.triples.get(graph_id) {
                        if let Some(stored) = graph.get(*idx) {
                            candidates.push(stored);
                        }
                    }
                }
            }
        } else if let Some(obj) = object {
            if let Some(indices) = self.object_index.get(obj) {
                for (graph_id, idx) in indices {
                    if let Some(graph) = self.triples.get(graph_id) {
                        if let Some(stored) = graph.get(*idx) {
                            candidates.push(stored);
                        }
                    }
                }
            }
        } else {
            // No pattern - return all triples
            for graph in self.triples.values() {
                for stored in graph {
                    candidates.push(stored);
                }
            }
        }

        // Filter candidates by remaining constraints
        candidates.into_iter().filter(|stored| {
            if let Some(s) = subject {
                if stored.triple.subject != s { return false; }
            }
            if let Some(p) = predicate {
                if stored.triple.predicate != p { return false; }
            }
            if let Some(o) = object {
                if stored.triple.object != o { return false; }
            }
            true
        }).collect()
    }

    /// Get all triples in a specific graph
    pub fn get_graph(&self, graph_id: &GraphId) -> Vec<&StoredTriple> {
        self.triples.get(graph_id)
            .map(|graph| graph.iter().collect())
            .unwrap_or_default()
    }

    /// Get all graph IDs
    pub fn graph_ids(&self) -> Vec<&GraphId> {
        self.triples.keys().collect()
    }

    /// Clear a specific graph
    pub fn clear_graph(&mut self, graph_id: &GraphId) {
        if let Some(graph) = self.triples.remove(graph_id) {
            let count = graph.len();

            // Remove from indices
            self.rebuild_indices();

        // Audit trail with memory management
        self.add_audit_entry(AuditEntry {
            id: format!("audit-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            operation: AuditOperation::Clear {
                graph_id: graph_id.clone(),
                triple_count: count,
            },
            actor: None,
            metadata: HashMap::new(),
        });
        }
    }

    /// Clear all graphs
    pub fn clear_all(&mut self) {
        let total_count: usize = self.triples.values().map(|g| g.len()).sum();

        self.triples.clear();
        self.subject_index.clear();
        self.predicate_index.clear();
        self.object_index.clear();

        // Audit trail with memory management
        self.add_audit_entry(AuditEntry {
            id: format!("audit-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            operation: AuditOperation::Clear {
                graph_id: GraphId::Default,
                triple_count: total_count,
            },
            actor: None,
            metadata: HashMap::new(),
        });
    }

    /// Get audit trail
    pub fn audit_trail(&self) -> &[AuditEntry] {
        &self.audit_trail
    }

    /// Get statistics
    pub fn statistics(&self) -> StoreStatistics {
        let total_triples: usize = self.triples.values().map(|g| g.len()).sum();
        let graph_count = self.triples.len();

        StoreStatistics {
            total_triples,
            graph_count,
            audit_entries: self.audit_trail.len(),
        }
    }

    /// Get all triples (for serialization)
    pub fn all_triples(&self) -> &HashMap<GraphId, Vec<StoredTriple>> {
        &self.triples
    }

    /// Get audit trail (for serialization)
    pub fn get_audit_trail(&self) -> &[AuditEntry] {
        &self.audit_trail
    }

    /// Add audit entry with memory management
    fn add_audit_entry(&mut self, entry: AuditEntry) {
        self.audit_trail.push(entry);

        // Memory management: remove oldest entries if over limit
        if self.audit_trail.len() > self.max_audit_entries {
            let remove_count = self.audit_trail.len() - self.max_audit_entries;
            self.audit_trail.drain(0..remove_count);
        }
    }

    /// Set maximum audit trail size
    pub fn set_audit_limit(&mut self, limit: usize) {
        self.max_audit_entries = limit;

        // Apply limit immediately if current size exceeds
        if self.audit_trail.len() > limit {
            let remove_count = self.audit_trail.len() - limit;
            self.audit_trail.drain(0..remove_count);
        }
    }

    /// Rebuild all indices (expensive operation)
    fn rebuild_indices(&mut self) {
        self.subject_index.clear();
        self.predicate_index.clear();
        self.object_index.clear();

        for (graph_id, graph) in &self.triples {
            for (idx, stored) in graph.iter().enumerate() {
                self.subject_index.entry(stored.triple.subject.clone())
                    .or_insert_with(HashSet::new)
                    .insert((graph_id.clone(), idx));
                self.predicate_index.entry(stored.triple.predicate.clone())
                    .or_insert_with(HashSet::new)
                    .insert((graph_id.clone(), idx));
                self.object_index.entry(stored.triple.object.clone())
                    .or_insert_with(HashSet::new)
                    .insert((graph_id.clone(), idx));
            }
        }
    }
}

/// Store statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStatistics {
    pub total_triples: usize,
    pub graph_count: usize,
    pub audit_entries: usize,
}

impl Default for RdfStore {
    fn default() -> Self {
        Self::new()
    }
}
