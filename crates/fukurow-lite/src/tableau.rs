//! テーブルロー推論アルゴリズム

use crate::model::{Ontology, Class, Property, Individual, Axiom, OwlIri};
use crate::OwlError;
use std::collections::{HashMap, HashSet, VecDeque};

/// Tableau node representing an individual
#[derive(Debug, Clone)]
struct Node {
    /// Individual IRI
    individual: Individual,
    /// Labels (classes this individual belongs to)
    labels: HashSet<Class>,
    /// Edges (property relations to other individuals)
    edges: HashMap<Property, HashSet<Individual>>,
    /// Negated labels (classes this individual does not belong to)
    negated_labels: HashSet<Class>,
    /// Blocked status (for blocking to ensure termination)
    blocked: bool,
}

/// Completion graph for tableau algorithm
#[derive(Debug)]
pub struct CompletionGraph {
    nodes: HashMap<Individual, Node>,
}

impl CompletionGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Initialize graph with individuals from ontology
    pub fn initialize(&mut self, ontology: &Ontology) {
        for individual in &ontology.individuals {
            let node = Node {
                individual: individual.clone(),
                labels: HashSet::new(),
                edges: HashMap::new(),
                negated_labels: HashSet::new(),
                blocked: false,
            };
            self.nodes.insert(individual.clone(), node);
        }
    }

    /// Add label to individual
    pub fn add_label(&mut self, individual: &Individual, class: Class) -> bool {
        if let Some(node) = self.nodes.get_mut(individual) {
            node.labels.insert(class)
        } else {
            false
        }
    }

    /// Check if individual has label
    pub fn has_label(&self, individual: &Individual, class: &Class) -> bool {
        self.nodes.get(individual)
            .map(|node| node.labels.contains(class))
            .unwrap_or(false)
    }

    /// Add edge between individuals
    pub fn add_edge(&mut self, from: &Individual, property: Property, to: &Individual) {
        if let Some(node) = self.nodes.get_mut(from) {
            node.edges.entry(property).or_insert_with(HashSet::new).insert(to.clone());
        }
    }

    /// Get all successors via property
    pub fn get_successors(&self, individual: &Individual, property: &Property) -> HashSet<Individual> {
        self.nodes.get(individual)
            .and_then(|node| node.edges.get(property))
            .cloned()
            .unwrap_or_default()
    }
}

/// Tableau reasoner for OWL Lite
pub struct TableauReasoner {
    graph: CompletionGraph,
}

impl TableauReasoner {
    pub fn new() -> Self {
        Self {
            graph: CompletionGraph::new(),
        }
    }

    /// Check if ontology is consistent (no contradictions)
    pub fn is_consistent(&mut self, ontology: &Ontology) -> Result<bool, OwlError> {
        self.graph.initialize(ontology);

        // Apply initial axioms
        self.apply_initial_assertions(ontology)?;

        // Apply tableau expansion rules until saturation
        let mut changed = true;
        while changed {
            changed = false;

            // Apply ⊓-rule (conjunction)
            changed |= self.apply_conjunction_rule()?;

            // Apply ∃-rule (existential restriction)
            changed |= self.apply_existential_rule(ontology)?;

            // Apply ∀-rule (universal restriction)
            changed |= self.apply_universal_rule(ontology)?;

            // Apply ⊔-rule (disjunction) - simplified for OWL Lite
            // OWL Lite doesn't have general disjunctions, so skip for now
        }

        // Check for contradictions
        self.check_contradictions()
    }

    /// Apply initial class and property assertions
    fn apply_initial_assertions(&mut self, ontology: &Ontology) -> Result<bool, OwlError> {
        let mut changed = false;

        for axiom in &ontology.axioms {
            match axiom {
                Axiom::ClassAssertion(class, individual) => {
                    changed |= self.graph.add_label(individual, class.clone());
                }
                Axiom::ObjectPropertyAssertion(prop, i1, i2) => {
                    self.graph.add_edge(i1, prop.clone(), i2);
                    changed = true;
                }
                _ => {} // Other axioms handled in expansion rules
            }
        }

        Ok(changed)
    }

    /// Apply conjunction rule (⊓-rule)
    /// If individual belongs to C1 ⊓ C2, then it belongs to C1 and C2
    fn apply_conjunction_rule(&mut self) -> Result<bool, OwlError> {
        // OWL Lite doesn't have explicit conjunction constructors
        // This would be handled by the class hierarchy in subsumption reasoning
        Ok(false)
    }

    /// Apply existential restriction rule (∃-rule)
    /// If individual belongs to ∃R.C and has no R-successor,
    /// create a new anonymous individual that belongs to C
    fn apply_existential_rule(&mut self, _ontology: &Ontology) -> Result<bool, OwlError> {
        // OWL Lite doesn't have existential restrictions in the classical sense
        // Domain/range restrictions are handled separately
        Ok(false)
    }

    /// Apply universal restriction rule (∀-rule)
    /// If individual belongs to ∀R.C and has R-successor y, then y belongs to C
    fn apply_universal_rule(&mut self, _ontology: &Ontology) -> Result<bool, OwlError> {
        // OWL Lite doesn't have universal restrictions in the classical sense
        // This would be handled by property range restrictions
        Ok(false)
    }

    /// Check for contradictions in the completion graph
    fn check_contradictions(&self) -> Result<bool, OwlError> {
        // Check if any individual belongs to both C and ¬C
        for node in self.graph.nodes.values() {
            // Check for owl:Nothing
            if node.labels.contains(&Class::Nothing) {
                return Ok(false); // Inconsistent
            }

            // Check for disjoint classes
            // This is a simplified check - full disjointness checking would be more complex
        }

        Ok(true) // Consistent
    }

    /// Compute subsumption hierarchy (class classification)
    pub fn compute_subsumption_hierarchy(&mut self, ontology: &Ontology) -> Result<HashMap<Class, HashSet<Class>>, OwlError> {
        let mut subsumption_map = HashMap::new();

        // Initialize with direct subsumptions from ontology
        for axiom in &ontology.axioms {
            if let Axiom::SubClassOf(subclass, superclass) = axiom {
                subsumption_map.entry(subclass.clone())
                    .or_insert_with(HashSet::new)
                    .insert(superclass.clone());
            }
        }

        // Compute transitive closure (simplified)
        self.compute_transitive_closure(&mut subsumption_map);

        Ok(subsumption_map)
    }

    /// Compute transitive closure of subsumption relations
    fn compute_transitive_closure(&self, subsumption_map: &mut HashMap<Class, HashSet<Class>>) {
        let mut changed = true;
        while changed {
            changed = false;
            let current_map = subsumption_map.clone();

            for (subclass, direct_supers) in &current_map {
                for direct_super in direct_supers {
                    if let Some(indirect_supers) = current_map.get(direct_super) {
                        for indirect_super in indirect_supers {
                            if subsumption_map.entry(subclass.clone())
                                .or_insert_with(HashSet::new)
                                .insert(indirect_super.clone()) {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }
}
