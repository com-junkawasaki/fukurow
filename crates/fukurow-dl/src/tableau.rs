//! OWL DL 拡張テーブルローアルゴリズム

use crate::model::{OwlDlOntology, ClassExpression, PropertyExpression, Axiom};
use fukurow_lite::Individual;
use crate::OwlDlError;
use fukurow_lite::tableau::{CompletionGraph, TableauReasoner};
use fukurow_lite::model::{OwlIri, Class, Property};
use std::collections::{HashMap, HashSet, VecDeque};

/// Extended completion graph for OWL DL (includes individual reasoning)
#[derive(Debug)]
pub struct DlCompletionGraph {
    /// Base completion graph from OWL Lite
    base_graph: CompletionGraph,

    /// Individual relationships and constraints
    individual_labels: HashMap<Individual, HashSet<ClassExpression>>,

    /// Property assertions between individuals
    property_assertions: HashMap<(Individual, PropertyExpression), HashSet<Individual>>,

    /// Negative property assertions
    negative_property_assertions: HashSet<(Individual, PropertyExpression, Individual)>,

    /// Same individual relationships
    same_individuals: Vec<HashSet<Individual>>,

    /// Different individuals
    different_individuals: HashSet<(Individual, Individual)>,
}

impl DlCompletionGraph {
    pub fn new() -> Self {
        Self {
            base_graph: CompletionGraph::new(),
            individual_labels: HashMap::new(),
            property_assertions: HashMap::new(),
            negative_property_assertions: HashSet::new(),
            same_individuals: Vec::new(),
            different_individuals: HashSet::new(),
        }
    }

    /// Initialize with ontology individuals
    pub fn initialize(&mut self, ontology: &OwlDlOntology) {
        for individual in &ontology.individuals {
            self.individual_labels.entry(individual.clone())
                .or_insert_with(HashSet::new);
        }
    }

    /// Add class expression label to individual
    pub fn add_label(&mut self, individual: &Individual, class_expr: ClassExpression) -> bool {
        self.individual_labels.entry(individual.clone())
            .or_insert_with(HashSet::new)
            .insert(class_expr)
    }

    /// Check if individual has class expression label
    pub fn has_label(&self, individual: &Individual, class_expr: &ClassExpression) -> bool {
        self.individual_labels.get(individual)
            .map(|labels| labels.contains(class_expr))
            .unwrap_or(false)
    }

    /// Add property assertion between individuals
    pub fn add_property_assertion(&mut self, from: &Individual, property: PropertyExpression, to: &Individual) {
        self.property_assertions.entry((from.clone(), property))
            .or_insert_with(HashSet::new)
            .insert(to.clone());
    }

    /// Get property successors
    pub fn get_property_successors(&self, individual: &Individual, property: &PropertyExpression) -> HashSet<Individual> {
        self.property_assertions.get(&(individual.clone(), property.clone()))
            .cloned()
            .unwrap_or_default()
    }

    /// Add same individual relationship
    pub fn add_same_individual(&mut self, i1: Individual, i2: Individual) {
        // Find existing equivalence classes
        let mut class1_idx = None;
        let mut class2_idx = None;

        for (idx, eq_class) in self.same_individuals.iter().enumerate() {
            if eq_class.contains(&i1) {
                class1_idx = Some(idx);
            }
            if eq_class.contains(&i2) {
                class2_idx = Some(idx);
            }
        }

        match (class1_idx, class2_idx) {
            (Some(idx1), Some(idx2)) => {
                if idx1 != idx2 {
                    // Merge equivalence classes
                    let removed_class = self.same_individuals.remove(idx2);
                    if let Some(class1) = self.same_individuals.get_mut(idx1) {
                        class1.extend(removed_class);
                    }
                }
            }
            (Some(idx), None) => {
                self.same_individuals[idx].insert(i2);
            }
            (None, Some(idx)) => {
                self.same_individuals[idx].insert(i1);
            }
            (None, None) => {
                let mut new_class = HashSet::new();
                new_class.insert(i1);
                new_class.insert(i2);
                self.same_individuals.push(new_class);
            }
        }
    }

    /// Add different individuals
    pub fn add_different_individuals(&mut self, i1: Individual, i2: Individual) {
        self.different_individuals.insert((i1.clone(), i2.clone()));
        self.different_individuals.insert((i2, i1)); // Symmetric
    }

    /// Check if individuals are the same
    pub fn are_same_individual(&self, i1: &Individual, i2: &Individual) -> bool {
        for eq_class in &self.same_individuals {
            if eq_class.contains(i1) && eq_class.contains(i2) {
                return true;
            }
        }
        false
    }

    /// Check if individuals are different
    pub fn are_different_individuals(&self, i1: &Individual, i2: &Individual) -> bool {
        self.different_individuals.contains(&(i1.clone(), i2.clone()))
    }
}

/// OWL DL Tableau reasoner
pub struct DlTableauReasoner {
    graph: DlCompletionGraph,
}

impl DlTableauReasoner {
    pub fn new() -> Self {
        Self {
            graph: DlCompletionGraph::new(),
        }
    }

    /// Check if OWL DL ontology is consistent
    pub fn is_consistent(&mut self, ontology: &OwlDlOntology) -> Result<bool, OwlDlError> {
        self.graph.initialize(ontology);

        // Apply initial assertions
        self.apply_initial_assertions(ontology)?;

        // Apply tableau expansion rules until saturation
        let mut changed = true;
        let mut iteration_count = 0;
        let max_iterations = 10000; // Prevent infinite loops

        while changed && iteration_count < max_iterations {
            changed = false;

            // Apply ⊓-rule (intersection)
            changed |= self.apply_intersection_rule()?;

            // Apply ⊔-rule (union) - simplified for OWL DL
            changed |= self.apply_union_rule()?;

            // Apply ∃-rule (existential restriction)
            changed |= self.apply_existential_rule(ontology)?;

            // Apply ∀-rule (universal restriction)
            changed |= self.apply_universal_rule(ontology)?;

            // Apply same/different individual rules
            changed |= self.apply_individual_rules()?;

            // Apply cardinality rules
            changed |= self.apply_cardinality_rules()?;

            iteration_count += 1;
        }

        if iteration_count >= max_iterations {
            return Err(OwlDlError::ReasoningError("Tableau algorithm did not terminate".to_string()));
        }

        // Check for contradictions
        self.check_contradictions()
    }

    /// Apply initial assertions from ontology
    fn apply_initial_assertions(&mut self, ontology: &OwlDlOntology) -> Result<bool, OwlDlError> {
        let mut changed = false;

        for axiom in &ontology.axioms {
            match axiom {
                Axiom::OwlLite(fukurow_lite::Axiom::ClassAssertion(class, individual)) => {
                    let expr = Self::owl_lite_class_to_expression(class.clone());
                    changed |= self.graph.add_label(individual, expr);
                }
                Axiom::OwlLite(fukurow_lite::Axiom::ObjectPropertyAssertion(prop, i1, i2)) => {
                    let expr = Self::owl_lite_property_to_expression(prop.clone());
                    self.graph.add_property_assertion(i1, expr, i2);
                    changed = true;
                }
                Axiom::OwlLite(fukurow_lite::Axiom::SameIndividual(individuals)) => {
                    for window in individuals.windows(2) {
                        if let [i1, i2] = window {
                            self.graph.add_same_individual(i1.clone(), i2.clone());
                            changed = true;
                        }
                    }
                }
                Axiom::OwlLite(fukurow_lite::Axiom::DifferentIndividuals(individuals)) => {
                    for i in individuals {
                        for j in individuals {
                            if i != j {
                                self.graph.add_different_individuals(i.clone(), j.clone());
                                changed = true;
                            }
                        }
                    }
                }
                Axiom::ClassAssertion(expr, individual) => {
                    changed |= self.graph.add_label(individual, expr.clone());
                }
                Axiom::ObjectPropertyAssertion(prop, i1, i2) => {
                    self.graph.add_property_assertion(i1, prop.clone(), i2);
                    changed = true;
                }
                Axiom::SameIndividual(individuals) => {
                    for window in individuals.windows(2) {
                        if let [i1, i2] = window {
                            self.graph.add_same_individual(i1.clone(), i2.clone());
                            changed = true;
                        }
                    }
                }
                Axiom::DifferentIndividuals(individuals) => {
                    for i in individuals {
                        for j in individuals {
                            if i != j {
                                self.graph.add_different_individuals(i.clone(), j.clone());
                                changed = true;
                            }
                        }
                    }
                }
                _ => {} // Other axioms handled in expansion rules
            }
        }

        Ok(changed)
    }

    /// Apply intersection rule (⊓-rule)
    fn apply_intersection_rule(&mut self) -> Result<bool, OwlDlError> {
        let mut changed = false;

        // For each individual, if it has an intersection class label,
        // add all constituent class labels
        let current_labels = self.graph.individual_labels.clone();

        for (individual, labels) in &current_labels {
            for label in labels {
                if let ClassExpression::IntersectionOf(constituents) = label {
                    for constituent in constituents {
                        if !self.graph.has_label(individual, constituent) {
                            self.graph.add_label(individual, constituent.clone());
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(changed)
    }

    /// Apply union rule (⊔-rule) - simplified
    fn apply_union_rule(&mut self) -> Result<bool, OwlDlError> {
        // OWL DL union reasoning is complex and typically requires disjunctive completion
        // For now, skip this rule (would require significant extension)
        Ok(false)
    }

    /// Apply existential restriction rule (∃-rule)
    fn apply_existential_rule(&mut self, _ontology: &OwlDlOntology) -> Result<bool, OwlDlError> {
        let mut changed = false;

        // For each individual with ∃R.C label, ensure R-successor with C label exists
        let current_labels = self.graph.individual_labels.clone();

        for (individual, labels) in &current_labels {
            for label in labels {
                if let ClassExpression::SomeValuesFrom { property, class } = label {
                    let successors = self.graph.get_property_successors(individual, property);

                    if successors.is_empty() {
                        // Create new anonymous individual
                        let new_individual = Individual(OwlIri::new(format!("anon_{}", individual.0.0)));
                        self.graph.add_label(&new_individual, *class.clone());
                        self.graph.add_property_assertion(individual, property.clone(), &new_individual);
                        changed = true;
                    } else {
                        // Ensure all successors have the required class label
                        for successor in &successors {
                            if !self.graph.has_label(successor, class) {
                                self.graph.add_label(successor, *class.clone());
                                changed = true;
                            }
                        }
                    }
                }
            }
        }

        Ok(changed)
    }

    /// Apply universal restriction rule (∀-rule)
    fn apply_universal_rule(&mut self, _ontology: &OwlDlOntology) -> Result<bool, OwlDlError> {
        let mut changed = false;

        // For each individual with ∀R.C label, propagate C to all R-successors
        let current_labels = self.graph.individual_labels.clone();

        for (individual, labels) in &current_labels {
            for label in labels {
                if let ClassExpression::AllValuesFrom { property, class } = label {
                    let successors = self.graph.get_property_successors(individual, property);

                    for successor in &successors {
                        if !self.graph.has_label(successor, class) {
                            self.graph.add_label(successor, *class.clone());
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(changed)
    }

    /// Apply individual reasoning rules
    fn apply_individual_rules(&mut self) -> Result<bool, OwlDlError> {
        let mut changed = false;

        // Apply same individual rules (merge labels)
        let same_individuals = self.graph.same_individuals.clone();
        for eq_class in &same_individuals {
            let individuals: Vec<_> = eq_class.iter().collect();

            for i in 1..individuals.len() {
                let source = individuals[0];
                let target = individuals[i];

                // Collect labels to merge first
                let source_labels = self.graph.individual_labels.get(source).cloned().unwrap_or_default();
                let source_property_assertions: Vec<_> = self.graph.property_assertions.iter()
                    .filter(|((from, _), _)| from == source)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                // Merge labels from source to target
                for label in &source_labels {
                    if !self.graph.has_label(target, label) {
                        self.graph.add_label(target, label.clone());
                        changed = true;
                    }
                }

                // Merge property assertions
                for ((_, prop), successors) in &source_property_assertions {
                    for successor in successors {
                        self.graph.add_property_assertion(target, prop.clone(), successor);
                        changed = true;
                    }
                }
            }
        }

        Ok(changed)
    }

    /// Apply cardinality rules
    fn apply_cardinality_rules(&mut self) -> Result<bool, OwlDlError> {
        // Cardinality reasoning is complex and typically requires counting
        // For now, implement basic min/max cardinality checking
        let mut changed = false;

        let current_labels = self.graph.individual_labels.clone();

        for (individual, labels) in &current_labels {
            for label in labels {
                match label {
                    ClassExpression::MinCardinality { cardinality, property, class } => {
                        let successors = self.graph.get_property_successors(individual, property);
                        let count = successors.len();

                        if count < *cardinality as usize {
                            // Need to create more successors
                            if let Some(class_expr) = class {
                                let new_individual = Individual(OwlIri::new(format!("anon_min_{}_{}", individual.0.0, count)));
                                self.graph.add_label(&new_individual, *class_expr.clone());
                                self.graph.add_property_assertion(individual, property.clone(), &new_individual);
                                changed = true;
                            }
                        }
                    }
                    ClassExpression::MaxCardinality { cardinality, property, .. } => {
                        let successors = self.graph.get_property_successors(individual, property);
                        let count = successors.len();

                        if count > *cardinality as usize {
                            // This would be a contradiction, but we can't handle it yet
                            // In full tableau, this would cause backtracking
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(changed)
    }

    /// Check for contradictions in the completion graph
    fn check_contradictions(&self) -> Result<bool, OwlDlError> {
        // Check if any individual belongs to both C and ¬C
        for labels in self.graph.individual_labels.values() {
            if labels.contains(&ClassExpression::Thing) && labels.contains(&ClassExpression::Nothing) {
                return Ok(false); // Inconsistent
            }

            // Check for explicit contradictions in intersections
            // This is a simplified check
        }

        // Check same/different individual contradictions
        for eq_class in &self.graph.same_individuals {
            let individuals: Vec<_> = eq_class.iter().collect();
            for i in 0..individuals.len() {
                for j in (i+1)..individuals.len() {
                    if self.graph.are_different_individuals(individuals[i], individuals[j]) {
                        return Ok(false); // Same individuals cannot be different
                    }
                }
            }
        }

        Ok(true) // Consistent
    }

    fn owl_lite_class_to_expression(class: Class) -> ClassExpression {
        match class {
            Class::Named(iri) => ClassExpression::Named(iri),
            Class::Thing => ClassExpression::Thing,
            Class::Nothing => ClassExpression::Nothing,
        }
    }

    fn owl_lite_property_to_expression(property: Property) -> PropertyExpression {
        match property {
            Property::Object(iri) => PropertyExpression::ObjectProperty(iri),
            Property::Data(iri) => PropertyExpression::DataProperty(iri),
        }
    }
}
