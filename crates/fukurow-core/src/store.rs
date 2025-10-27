//! Graph storage and manipulation

use crate::model::{Triple, NamedGraph, JsonLdDocument};
use std::collections::HashMap;
use anyhow::Result;
use smallvec::SmallVec;

/// In-memory graph store with indexing for fast queries
#[derive(Debug, Clone, Default)]
pub struct GraphStore {
    /// Named graphs for organization
    graphs: HashMap<String, NamedGraph>,
    /// Default graph for unnamed triples
    default_graph: NamedGraph,
    /// Subject index: subject -> list of triple indices in default_graph.triples
    subject_index: HashMap<String, SmallVec<[usize; 8]>>,
    /// Predicate index: predicate -> list of triple indices
    predicate_index: HashMap<String, SmallVec<[usize; 8]>>,
    /// Object index: object -> list of triple indices
    object_index: HashMap<String, SmallVec<[usize; 8]>>,
}

impl GraphStore {
    pub fn new() -> Self {
        Self {
            graphs: HashMap::new(),
            default_graph: NamedGraph {
                name: "default".to_string(),
                triples: Vec::new(),
            },
            subject_index: HashMap::new(),
            predicate_index: HashMap::new(),
            object_index: HashMap::new(),
        }
    }

    /// Add a triple to the default graph
    pub fn add_triple(&mut self, triple: Triple) {
        let index = self.default_graph.triples.len();
        self.default_graph.triples.push(triple.clone());

        // Update indices
        self.subject_index.entry(triple.subject.clone()).or_insert_with(SmallVec::new).push(index);
        self.predicate_index.entry(triple.predicate.clone()).or_insert_with(SmallVec::new).push(index);
        self.object_index.entry(triple.object.clone()).or_insert_with(SmallVec::new).push(index);
    }

    /// Add a triple to a named graph
    pub fn add_triple_to_graph(&mut self, graph_name: &str, triple: Triple) {
        self.graphs
            .entry(graph_name.to_string())
            .or_insert_with(|| NamedGraph {
                name: graph_name.to_string(),
                triples: Vec::new(),
            })
            .triples
            .push(triple);
    }

    /// Add multiple triples to a named graph
    pub fn add_triples_to_graph(&mut self, graph_name: &str, triples: Vec<Triple>) {
        self.graphs
            .entry(graph_name.to_string())
            .or_insert_with(|| NamedGraph {
                name: graph_name.to_string(),
                triples: Vec::new(),
            })
            .triples
            .extend(triples);
    }

    /// Get all triples from default graph
    pub fn get_default_graph(&self) -> &NamedGraph {
        &self.default_graph
    }

    /// Get a named graph
    pub fn get_graph(&self, name: &str) -> Option<&NamedGraph> {
        self.graphs.get(name)
    }

    /// Get all graph names
    pub fn graph_names(&self) -> Vec<String> {
        self.graphs.keys().cloned().collect()
    }

    /// Find triples matching a pattern (SPARQL-like) - optimized with indexing
    pub fn find_triples(&self, subject: Option<&str>, predicate: Option<&str>, object: Option<&str>) -> Vec<&Triple> {
        // Use the most selective index to minimize the search space
        let candidate_indices: SmallVec<[usize; 8]> = match (subject, predicate, object) {
            // Exact triple lookup - use SPO intersection
            (Some(s), Some(p), Some(o)) => {
                self.find_exact_triple_indices(s, p, o)
            },
            // Single component lookups - use the most selective index
            (Some(s), None, None) => {
                self.subject_index.get(s).cloned().unwrap_or(SmallVec::new())
            },
            (None, Some(p), None) => {
                self.predicate_index.get(p).cloned().unwrap_or(SmallVec::new())
            },
            (None, None, Some(o)) => {
                self.object_index.get(o).cloned().unwrap_or(SmallVec::new())
            },
            // Two-component patterns - intersect the two indices
            (Some(s), Some(p), None) => {
                self.intersect_indices(
                    self.subject_index.get(s).map(|v| v.as_slice()).unwrap_or(&[]),
                    self.predicate_index.get(p).map(|v| v.as_slice()).unwrap_or(&[]),
                )
            },
            (Some(s), None, Some(o)) => {
                self.intersect_indices(
                    self.subject_index.get(s).map(|v| v.as_slice()).unwrap_or(&[]),
                    self.object_index.get(o).map(|v| v.as_slice()).unwrap_or(&[]),
                )
            },
            (None, Some(p), Some(o)) => {
                self.intersect_indices(
                    self.predicate_index.get(p).map(|v| v.as_slice()).unwrap_or(&[]),
                    self.object_index.get(o).map(|v| v.as_slice()).unwrap_or(&[]),
                )
            },
            // No constraints - return all indices
            (None, None, None) => {
                (0..self.default_graph.triples.len()).map(|i| i).collect()
            }
        };

        // Convert indices to triple references
        let mut results = Vec::new();
        for &index in &candidate_indices {
            if let Some(triple) = self.default_graph.triples.get(index) {
                results.push(triple);
            }
        }

        results
    }

    /// Find indices of triples that exactly match SPO
    fn find_exact_triple_indices(&self, subject: &str, predicate: &str, object: &str) -> SmallVec<[usize; 8]> {
        if let Some(subject_indices) = self.subject_index.get(subject) {
            if let Some(predicate_indices) = self.predicate_index.get(predicate) {
                if let Some(object_indices) = self.object_index.get(object) {
                    // Intersect all three indices
                    let mut result = SmallVec::new();
                    for &idx in subject_indices {
                        if predicate_indices.contains(&idx) && object_indices.contains(&idx) {
                            result.push(idx);
                        }
                    }
                    return result;
                }
            }
        }
        SmallVec::new()
    }

    /// Intersect two sorted index vectors
    fn intersect_indices(&self, a: &[usize], b: &[usize]) -> SmallVec<[usize; 8]> {
        let mut result = SmallVec::new();
        let mut i = 0;
        let mut j = 0;

        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    result.push(a[i]);
                    i += 1;
                    j += 1;
                }
            }
        }

        result
    }

    /// Convert store to JSON-LD document
    pub fn to_jsonld(&self) -> Result<JsonLdDocument> {
        let mut graph = Vec::new();

        // Add default graph triples
        for triple in &self.default_graph.triples {
            let node = serde_json::json!({
                "@id": triple.subject,
                triple.predicate.clone(): triple.object
            });
            graph.push(node);
        }

        // Add named graph triples
        for named_graph in self.graphs.values() {
            for triple in &named_graph.triples {
                let node = serde_json::json!({
                    "@id": triple.subject,
                    triple.predicate.clone(): triple.object
                });
                graph.push(node);
            }
        }

        Ok(JsonLdDocument {
            context: serde_json::json!({
                "@vocab": "https://w3id.org/security#",
                "rdf": "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
                "rdfs": "http://www.w3.org/2000/01/rdf-schema#"
            }),
            graph: Some(graph),
            data: HashMap::new(),
        })
    }

    /// Clear all graphs
    pub fn clear(&mut self) {
        self.graphs.clear();
        self.default_graph.triples.clear();
        self.subject_index.clear();
        self.predicate_index.clear();
        self.object_index.clear();
    }
}
