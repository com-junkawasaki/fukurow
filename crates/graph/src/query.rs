//! Query interface for graph operations

use crate::model::Triple;
use crate::store::GraphStore;
use std::collections::HashSet;

/// Simple query builder for graph patterns
#[derive(Debug, Clone)]
pub struct GraphQuery {
    patterns: Vec<QueryPattern>,
}

#[derive(Debug, Clone)]
pub struct QueryPattern {
    pub subject: PatternValue,
    pub predicate: PatternValue,
    pub object: PatternValue,
}

#[derive(Debug, Clone)]
pub enum PatternValue {
    Variable(String),
    Constant(String),
}

impl GraphQuery {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    pub fn where_clause(mut self, subject: PatternValue, predicate: PatternValue, object: PatternValue) -> Self {
        self.patterns.push(QueryPattern {
            subject,
            predicate,
            object,
        });
        self
    }

    /// Execute query against the graph store
    pub fn execute(&self, store: &GraphStore) -> Vec<HashMap<String, String>> {
        let mut results = Vec::new();
        self.execute_recursive(store, 0, &mut HashMap::new(), &mut results);
        results
    }

    fn execute_recursive(
        &self,
        store: &GraphStore,
        pattern_index: usize,
        bindings: &mut HashMap<String, String>,
        results: &mut Vec<HashMap<String, String>>,
    ) {
        if pattern_index >= self.patterns.len() {
            // All patterns matched, add result
            results.push(bindings.clone());
            return;
        }

        let pattern = &self.patterns[pattern_index];
        let matches = self.find_matches(store, pattern, bindings);

        for binding_update in matches {
            // Apply new bindings
            let mut new_bindings = bindings.clone();
            for (var, value) in binding_update {
                new_bindings.insert(var, value);
            }

            // Continue with next pattern
            self.execute_recursive(store, pattern_index + 1, &mut new_bindings, results);
        }
    }

    fn find_matches(
        &self,
        store: &GraphStore,
        pattern: &QueryPattern,
        current_bindings: &HashMap<String, String>,
    ) -> Vec<HashMap<String, String>> {
        let subject = self.resolve_pattern_value(&pattern.subject, current_bindings);
        let predicate = self.resolve_pattern_value(&pattern.predicate, current_bindings);
        let object = self.resolve_pattern_value(&pattern.object, current_bindings);

        let matches = store.find_triples(subject.as_deref(), predicate.as_deref(), object.as_deref());

        let mut results = Vec::new();

        for triple in matches {
            let mut binding = HashMap::new();

            // Bind variables
            if let PatternValue::Variable(var) = &pattern.subject {
                if !current_bindings.contains_key(var) {
                    binding.insert(var.clone(), triple.subject.clone());
                }
            }
            if let PatternValue::Variable(var) = &pattern.predicate {
                if !current_bindings.contains_key(var) {
                    binding.insert(var.clone(), triple.predicate.clone());
                }
            }
            if let PatternValue::Variable(var) = &pattern.object {
                if !current_bindings.contains_key(var) {
                    binding.insert(var.clone(), triple.object.clone());
                }
            }

            // Check if binding is consistent with current bindings
            let mut consistent = true;
            for (var, value) in &binding {
                if let Some(existing) = current_bindings.get(var) {
                    if existing != value {
                        consistent = false;
                        break;
                    }
                }
            }

            if consistent {
                results.push(binding);
            }
        }

        results
    }

    fn resolve_pattern_value(&self, value: &PatternValue, bindings: &HashMap<String, String>) -> Option<String> {
        match value {
            PatternValue::Constant(s) => Some(s.clone()),
            PatternValue::Variable(var) => bindings.get(var).cloned(),
        }
    }
}

/// Helper functions for creating patterns
pub fn var(name: &str) -> PatternValue {
    PatternValue::Variable(name.to_string())
}

pub fn const_val(value: &str) -> PatternValue {
    PatternValue::Constant(value.to_string())
}
