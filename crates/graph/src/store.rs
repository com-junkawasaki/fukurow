//! Graph storage and manipulation

use crate::model::{Triple, NamedGraph, JsonLdDocument};
use std::collections::{HashMap, HashSet};
use anyhow::{Result, anyhow};

/// In-memory graph store
#[derive(Debug, Clone, Default)]
pub struct GraphStore {
    /// Named graphs for organization
    graphs: HashMap<String, NamedGraph>,
    /// Default graph for unnamed triples
    default_graph: NamedGraph,
}

impl GraphStore {
    pub fn new() -> Self {
        Self {
            graphs: HashMap::new(),
            default_graph: NamedGraph {
                name: "default".to_string(),
                triples: Vec::new(),
            },
        }
    }

    /// Add a triple to the default graph
    pub fn add_triple(&mut self, triple: Triple) {
        self.default_graph.triples.push(triple);
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

    /// Find triples matching a pattern (SPARQL-like)
    pub fn find_triples(&self, subject: Option<&str>, predicate: Option<&str>, object: Option<&str>) -> Vec<&Triple> {
        let mut results = Vec::new();

        // Search in default graph
        for triple in &self.default_graph.triples {
            if self.matches_pattern(triple, subject, predicate, object) {
                results.push(triple);
            }
        }

        // Search in named graphs
        for graph in self.graphs.values() {
            for triple in &graph.triples {
                if self.matches_pattern(triple, subject, predicate, object) {
                    results.push(triple);
                }
            }
        }

        results
    }

    fn matches_pattern(&self, triple: &Triple, subject: Option<&str>, predicate: Option<&str>, object: Option<&str>) -> bool {
        if let Some(s) = subject {
            if triple.subject != s {
                return false;
            }
        }
        if let Some(p) = predicate {
            if triple.predicate != p {
                return false;
            }
        }
        if let Some(o) = object {
            if triple.object != o {
                return false;
            }
        }
        true
    }

    /// Convert store to JSON-LD document
    pub fn to_jsonld(&self) -> Result<JsonLdDocument> {
        let mut graph = Vec::new();

        // Add default graph triples
        for triple in &self.default_graph.triples {
            let mut node = serde_json::json!({
                "@id": triple.subject,
                triple.predicate.clone(): triple.object
            });
            graph.push(node);
        }

        // Add named graph triples
        for named_graph in self.graphs.values() {
            for triple in &named_graph.triples {
                let mut node = serde_json::json!({
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
    }
}
