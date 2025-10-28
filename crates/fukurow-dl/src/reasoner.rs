//! OWL DL リーナー

use crate::model::{OwlDlOntology, ClassExpression, PropertyExpression, Axiom};
use crate::loader::OwlDlOntologyLoader;
use crate::tableau::DlTableauReasoner;
use crate::OwlDlError;
use fukurow_store::store::RdfStore;
use fukurow_lite::{OwlLiteReasoner, Ontology as OwlLiteOntology, model::OwlIri};
use std::collections::{HashMap, HashSet};

/// OWL DL reasoner
pub struct OwlDlReasoner {
    lite_reasoner: OwlLiteReasoner,
    dl_tableau: DlTableauReasoner,
}

impl OwlDlReasoner {
    pub fn new() -> Self {
        Self {
            lite_reasoner: OwlLiteReasoner::new(),
            dl_tableau: DlTableauReasoner::new(),
        }
    }

    /// Load OWL DL ontology from RDF store
    pub fn load_ontology(&self, store: &RdfStore) -> Result<OwlDlOntology, OwlDlError> {
        // First try to load as OWL Lite
        let lite_ontology = self.lite_reasoner.load_ontology(store)
            .map_err(|e| OwlDlError::LoaderError(e.to_string()))?;

        // Convert to OWL DL ontology
        let mut dl_ontology = OwlDlOntology::new();

        // Copy basic information
        dl_ontology.iri = lite_ontology.iri;
        dl_ontology.classes = lite_ontology.classes;
        dl_ontology.properties = lite_ontology.properties;
        dl_ontology.individuals = lite_ontology.individuals;

        // Convert axioms
        for axiom in lite_ontology.axioms {
            dl_ontology.add_axiom(Axiom::OwlLite(axiom));
        }

        // TODO: Load additional OWL DL constructs from RDF
        // This would include intersectionOf, unionOf, restrictions, etc.

        Ok(dl_ontology)
    }

    /// Check if OWL DL ontology is consistent
    pub fn is_consistent(&mut self, ontology: &OwlDlOntology) -> Result<bool, OwlDlError> {
        self.dl_tableau.is_consistent(ontology)
    }

    /// Check if class expression C1 is subsumed by class expression C2 (C1 ⊑ C2)
    pub fn is_subsumed_by(&mut self, _ontology: &OwlDlOntology, _subclass: &ClassExpression, _superclass: &ClassExpression) -> Result<bool, OwlDlError> {
        // Subsumption reasoning for complex class expressions is very complex
        // This would require model construction or other advanced techniques
        Err(OwlDlError::UnsupportedFeature("Complex class expression subsumption not yet implemented".to_string()))
    }

    /// Get all subclasses of a class expression
    pub fn get_subclasses(&mut self, _ontology: &OwlDlOntology, _class: &ClassExpression) -> Result<HashSet<ClassExpression>, OwlDlError> {
        Err(OwlDlError::UnsupportedFeature("Subclass reasoning for complex expressions not yet implemented".to_string()))
    }

    /// Get all superclasses of a class expression
    pub fn get_superclasses(&mut self, _ontology: &OwlDlOntology, _class: &ClassExpression) -> Result<HashSet<ClassExpression>, OwlDlError> {
        Err(OwlDlError::UnsupportedFeature("Superclass reasoning for complex expressions not yet implemented".to_string()))
    }

    /// Check if individual is instance of class expression
    pub fn is_instance_of(&mut self, ontology: &OwlDlOntology, individual: &fukurow_lite::Individual, class_expr: &ClassExpression) -> Result<bool, OwlDlError> {
        match class_expr {
            ClassExpression::Named(iri) => {
                // Check direct class assertions and subclass relationships
                self.check_named_class_membership(ontology, individual, iri)
            }
            ClassExpression::Thing => Ok(true),
            ClassExpression::Nothing => Ok(false),
            ClassExpression::IntersectionOf(classes) => {
                // Individual must be instance of ALL classes
                for class in classes {
                    if !self.is_instance_of(ontology, individual, class)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ClassExpression::UnionOf(classes) => {
                // Individual must be instance of AT LEAST ONE class
                for class in classes {
                    if self.is_instance_of(ontology, individual, class)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ClassExpression::ComplementOf(class) => {
                // Individual is NOT an instance of the class
                let is_instance = self.is_instance_of(ontology, individual, class)?;
                Ok(!is_instance)
            }
            ClassExpression::OneOf(individuals) => {
                // Individual is one of the enumerated individuals
                Ok(individuals.contains(individual))
            }
            ClassExpression::SomeValuesFrom { property, class } => {
                self.check_some_values_from(ontology, individual, property, class)
            }
            ClassExpression::AllValuesFrom { property, class } => {
                self.check_all_values_from(ontology, individual, property, class)
            }
            ClassExpression::HasValue { property, individual: value } => {
                self.check_has_value(ontology, individual, property, value)
            }
            ClassExpression::MinCardinality { cardinality, property, class } => {
                self.check_min_cardinality(ontology, individual, property, *cardinality, class.as_ref().map(|c| &**c))
            }
            ClassExpression::MaxCardinality { cardinality, property, class } => {
                self.check_max_cardinality(ontology, individual, property, *cardinality, class.as_ref().map(|c| &**c))
            }
            ClassExpression::ExactCardinality { cardinality, property, class } => {
                self.check_exact_cardinality(ontology, individual, property, *cardinality, class.as_ref().map(|c| &**c))
            }
        }
    }

    /// Check if individual is a member of a named class (including subclasses)
    fn check_named_class_membership(&mut self, ontology: &OwlDlOntology, individual: &fukurow_lite::Individual, class_iri: &OwlIri) -> Result<bool, OwlDlError> {
        let target_class = fukurow_lite::Class::Named(class_iri.clone());

        // First check direct assertions
        for axiom in &ontology.axioms {
            if let Axiom::OwlLite(fukurow_lite::Axiom::ClassAssertion(class, ind)) = axiom {
                if ind == individual && class == &target_class {
                    return Ok(true);
                }
            }
        }

        // Check subclass relationships - if individual is instance of subclass, it's also instance of superclass
        for axiom in &ontology.axioms {
            if let Axiom::OwlLite(fukurow_lite::Axiom::SubClassOf(subclass, superclass)) = axiom {
                if superclass == &target_class {
                    if let fukurow_lite::Class::Named(subclass_iri) = subclass {
                        let subclass_expr = ClassExpression::Named(subclass_iri.clone());
                        if self.is_instance_of(ontology, individual, &subclass_expr)? {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check ∃R.C restriction
    fn check_some_values_from(&mut self, ontology: &OwlDlOntology, individual: &fukurow_lite::Individual, property: &PropertyExpression, class: &ClassExpression) -> Result<bool, OwlDlError> {
        // Find all property assertions from this individual
        for axiom in &ontology.axioms {
            match axiom {
                Axiom::ObjectPropertyAssertion(prop_expr, subject, object) => {
                    if subject == individual && prop_expr == property {
                        // Check if object is instance of the required class
                        let object_individual = fukurow_lite::Individual(object.0.clone());
                        if self.is_instance_of(ontology, &object_individual, class)? {
                            return Ok(true);
                        }
                    }
                }
                Axiom::OwlLite(fukurow_lite::Axiom::ObjectPropertyAssertion(prop, subject, object)) => {
                    if let fukurow_lite::Property::Object(iri) = prop {
                        let prop_expr = PropertyExpression::ObjectProperty(iri.clone());
                        if subject == individual && &prop_expr == property {
                            // Check if object is instance of the required class
                            let object_individual = fukurow_lite::Individual(object.0.clone());
                            if self.is_instance_of(ontology, &object_individual, class)? {
                                return Ok(true);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(false)
    }

    /// Check ∀R.C restriction
    fn check_all_values_from(&mut self, ontology: &OwlDlOntology, individual: &fukurow_lite::Individual, property: &PropertyExpression, class: &ClassExpression) -> Result<bool, OwlDlError> {
        // Find all property assertions from this individual
        for axiom in &ontology.axioms {
            match axiom {
                Axiom::ObjectPropertyAssertion(prop_expr, subject, object) => {
                    if subject == individual && prop_expr == property {
                        // Check if ALL objects are instances of the required class
                        let object_individual = fukurow_lite::Individual(object.0.clone());
                        if !self.is_instance_of(ontology, &object_individual, class)? {
                            return Ok(false);
                        }
                    }
                }
                Axiom::OwlLite(fukurow_lite::Axiom::ObjectPropertyAssertion(prop, subject, object)) => {
                    if let fukurow_lite::Property::Object(iri) = prop {
                        let prop_expr = PropertyExpression::ObjectProperty(iri.clone());
                        if subject == individual && &prop_expr == property {
                            // Check if ALL objects are instances of the required class
                            let object_individual = fukurow_lite::Individual(object.0.clone());
                            if !self.is_instance_of(ontology, &object_individual, class)? {
                                return Ok(false);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(true)
    }

    /// Check ∃R.{i} restriction
    fn check_has_value(&mut self, ontology: &OwlDlOntology, individual: &fukurow_lite::Individual, property: &PropertyExpression, value: &fukurow_lite::Individual) -> Result<bool, OwlDlError> {
        // Check if there's a property assertion with the specific value
        for axiom in &ontology.axioms {
            match axiom {
                Axiom::ObjectPropertyAssertion(prop_expr, subject, object) => {
                    if subject == individual && prop_expr == property && object == value {
                        return Ok(true);
                    }
                }
                Axiom::OwlLite(fukurow_lite::Axiom::ObjectPropertyAssertion(prop, subject, object)) => {
                    if let fukurow_lite::Property::Object(iri) = prop {
                        let prop_expr = PropertyExpression::ObjectProperty(iri.clone());
                        if subject == individual && &prop_expr == property && object == value {
                            return Ok(true);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(false)
    }

    /// Helper to collect property assertions for an individual and property
    fn collect_property_assertions(&self, ontology: &OwlDlOntology, individual: &fukurow_lite::Individual, property: &PropertyExpression) -> Vec<fukurow_lite::Individual> {
        let mut objects = Vec::new();

        for axiom in &ontology.axioms {
            match axiom {
                Axiom::ObjectPropertyAssertion(prop_expr, subject, object) => {
                    if subject == individual && prop_expr == property {
                        objects.push(fukurow_lite::Individual(object.0.clone()));
                    }
                }
                Axiom::OwlLite(fukurow_lite::Axiom::ObjectPropertyAssertion(prop, subject, object)) => {
                    if let fukurow_lite::Property::Object(iri) = prop {
                        let prop_expr = PropertyExpression::ObjectProperty(iri.clone());
                        if subject == individual && &prop_expr == property {
                            objects.push(fukurow_lite::Individual(object.0.clone()));
                        }
                    }
                }
                _ => {}
            }
        }

        objects
    }

    /// Check ≥n R.C restriction
    fn check_min_cardinality(&mut self, ontology: &OwlDlOntology, individual: &fukurow_lite::Individual, property: &PropertyExpression, min_count: u32, class: Option<&ClassExpression>) -> Result<bool, OwlDlError> {
        let objects = self.collect_property_assertions(ontology, individual, property);
        let mut count = 0;

        // Count property assertions that satisfy the class restriction
        for object_individual in objects {
            // If class is specified, check if object satisfies it
            let satisfies_class = match class {
                Some(required_class) => self.is_instance_of(ontology, &object_individual, required_class)?,
                None => true, // No class restriction (owl:Thing)
            };

            if satisfies_class {
                count += 1;
            }
        }

        Ok(count >= min_count)
    }

    /// Check ≤n R.C restriction
    fn check_max_cardinality(&mut self, ontology: &OwlDlOntology, individual: &fukurow_lite::Individual, property: &PropertyExpression, max_count: u32, class: Option<&ClassExpression>) -> Result<bool, OwlDlError> {
        let objects = self.collect_property_assertions(ontology, individual, property);
        let mut count = 0;

        // Count property assertions that satisfy the class restriction
        for object_individual in objects {
            // If class is specified, check if object satisfies it
            let satisfies_class = match class {
                Some(required_class) => self.is_instance_of(ontology, &object_individual, required_class)?,
                None => true, // No class restriction (owl:Thing)
            };

            if satisfies_class {
                count += 1;
                if count > max_count {
                    return Ok(false);
                }
            }
        }

        Ok(count <= max_count)
    }

    /// Check =n R.C restriction
    fn check_exact_cardinality(&mut self, ontology: &OwlDlOntology, individual: &fukurow_lite::Individual, property: &PropertyExpression, exact_count: u32, class: Option<&ClassExpression>) -> Result<bool, OwlDlError> {
        let min_result = self.check_min_cardinality(ontology, individual, property, exact_count, class)?;
        let max_result = self.check_max_cardinality(ontology, individual, property, exact_count, class)?;

        Ok(min_result && max_result)
    }

    /// Classify ontology (compute class hierarchy for complex expressions)
    pub fn classify_ontology(&mut self, _ontology: &OwlDlOntology) -> Result<HashMap<ClassExpression, HashSet<ClassExpression>>, OwlDlError> {
        // Full classification of OWL DL ontologies is undecidable in general
        // We can only provide partial results for tractable cases
        Err(OwlDlError::UnsupportedFeature("Full OWL DL classification not yet implemented".to_string()))
    }

    /// Realize ontology (compute individual types for complex expressions)
    pub fn realize_ontology(&mut self, _ontology: &OwlDlOntology) -> Result<HashMap<fukurow_lite::Individual, HashSet<ClassExpression>>, OwlDlError> {
        Err(OwlDlError::UnsupportedFeature("Ontology realization for OWL DL not yet implemented".to_string()))
    }

    /// Get inferred axioms (closure of the ontology)
    pub fn get_inferred_axioms(&mut self, _ontology: &OwlDlOntology) -> Result<Vec<Axiom>, OwlDlError> {
        // Computing the full deductive closure of OWL DL is complex
        // For now, return empty set
        Ok(Vec::new())
    }

    /// Convert OWL DL ontology to OWL Lite (for compatibility)
    pub fn to_owl_lite(&self, dl_ontology: &OwlDlOntology) -> Result<OwlLiteOntology, OwlDlError> {
        let mut lite_ontology = OwlLiteOntology::new();

        // Copy basic information
        lite_ontology.iri = dl_ontology.iri.clone();
        lite_ontology.classes = dl_ontology.classes.clone();
        lite_ontology.properties = dl_ontology.properties.clone();
        lite_ontology.individuals = dl_ontology.individuals.clone();

        // Convert compatible axioms
        for axiom in &dl_ontology.axioms {
            match axiom {
                Axiom::OwlLite(lite_axiom) => {
                    lite_ontology.add_axiom(lite_axiom.clone());
                }
                Axiom::SubClassOf(ClassExpression::Named(sub_iri), ClassExpression::Named(super_iri)) => {
                    let sub_class = fukurow_lite::Class::Named(sub_iri.clone());
                    let super_class = fukurow_lite::Class::Named(super_iri.clone());
                    lite_ontology.add_axiom(fukurow_lite::Axiom::SubClassOf(sub_class, super_class));
                }
                Axiom::ClassAssertion(ClassExpression::Named(iri), individual) => {
                    let class = fukurow_lite::Class::Named(iri.clone());
                    lite_ontology.add_axiom(fukurow_lite::Axiom::ClassAssertion(class, individual.clone()));
                }
                Axiom::ObjectPropertyAssertion(PropertyExpression::ObjectProperty(iri), i1, i2) => {
                    let prop = fukurow_lite::Property::Object(iri.clone());
                    lite_ontology.add_axiom(fukurow_lite::Axiom::ObjectPropertyAssertion(prop, i1.clone(), i2.clone()));
                }
                // Skip complex axioms that can't be converted
                _ => {}
            }
        }

        Ok(lite_ontology)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_core::model::Triple;
    use fukurow_store::store::RdfStore;
    use fukurow_store::provenance::{Provenance, GraphId};
    use fukurow_lite::model::OwlIri;
    use fukurow_lite::Individual;

    fn create_test_store() -> RdfStore {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test".to_string());
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(1.0),
        };

        // Create a simple ontology with some OWL DL constructs
        let triples = vec![
            // Class declarations
            Triple {
                subject: "http://example.org/Person".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            Triple {
                subject: "http://example.org/Animal".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            // Subclass relations
            Triple {
                subject: "http://example.org/Person".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                object: "http://example.org/Animal".to_string(),
            },
            // Individual
            Triple {
                subject: "http://example.org/john".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            // Class assertion
            Triple {
                subject: "http://example.org/john".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://example.org/Person".to_string(),
            },
        ];

        for triple in triples {
            store.insert(triple, graph_id.clone(), provenance.clone());
        }

        store
    }

    #[test]
    fn test_load_dl_ontology() {
        let store = create_test_store();
        let reasoner = OwlDlReasoner::new();

        let ontology = reasoner.load_ontology(&store).unwrap();

        assert!(ontology.classes.contains(&fukurow_lite::Class::Named(OwlIri::new("http://example.org/Person".to_string()))));
        assert!(ontology.individuals.contains(&Individual(OwlIri::new("http://example.org/john".to_string()))));

        // Check for class assertion axiom
        let has_assertion = ontology.axioms.iter().any(|axiom| {
            matches!(axiom, Axiom::OwlLite(fukurow_lite::Axiom::ClassAssertion(
                fukurow_lite::Class::Named(iri), Individual(ind_iri)
            )) if iri.0 == "http://example.org/Person" && ind_iri.0 == "http://example.org/john")
        });
        assert!(has_assertion);
    }

    #[test]
    fn test_dl_consistency_check() {
        let store = create_test_store();
        let mut reasoner = OwlDlReasoner::new();

        let ontology = reasoner.load_ontology(&store).unwrap();
        let is_consistent = reasoner.is_consistent(&ontology).unwrap();

        assert!(is_consistent);
    }

    #[test]
    fn test_convert_to_owl_lite() {
        let store = create_test_store();
        let reasoner = OwlDlReasoner::new();

        let dl_ontology = reasoner.load_ontology(&store).unwrap();
        let lite_ontology = reasoner.to_owl_lite(&dl_ontology).unwrap();

        // Should have the same basic structure
        assert_eq!(dl_ontology.classes, lite_ontology.classes);
        assert_eq!(dl_ontology.individuals, lite_ontology.individuals);
    }

    #[test]
    fn test_intersection_of_reasoning() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test".to_string());
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(1.0),
        };

        // Create intersection class: Person ∩ Mammal
        let triples = vec![
            // Define classes
            Triple {
                subject: "http://example.org/Person".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            Triple {
                subject: "http://example.org/Mammal".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            // Define intersection class
            Triple {
                subject: "http://example.org/Human".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            Triple {
                subject: "http://example.org/Human".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#intersectionOf".to_string(),
                object: "http://example.org/HumanList".to_string(),
            },
            // List: Person AND Mammal
            Triple {
                subject: "http://example.org/HumanList".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#first".to_string(),
                object: "http://example.org/Person".to_string(),
            },
            Triple {
                subject: "http://example.org/HumanList".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest".to_string(),
                object: "http://example.org/MammalList".to_string(),
            },
            Triple {
                subject: "http://example.org/MammalList".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#first".to_string(),
                object: "http://example.org/Mammal".to_string(),
            },
            Triple {
                subject: "http://example.org/MammalList".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest".to_string(),
                object: "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil".to_string(),
            },
            // Individual
            Triple {
                subject: "http://example.org/alice".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/alice".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://example.org/Person".to_string(),
            },
            Triple {
                subject: "http://example.org/alice".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://example.org/Mammal".to_string(),
            },
        ];

        for triple in triples {
            store.insert(triple, graph_id.clone(), provenance.clone());
        }

        let mut reasoner = OwlDlReasoner::new();
        let ontology = reasoner.load_ontology(&store).unwrap();

        // Alice should be inferred to be a Human (Person ∩ Mammal)
        let alice = Individual(OwlIri::new("http://example.org/alice".to_string()));
        let human_class = ClassExpression::IntersectionOf(vec![
            ClassExpression::Named(OwlIri::new("http://example.org/Person".to_string())),
            ClassExpression::Named(OwlIri::new("http://example.org/Mammal".to_string())),
        ]);

        let is_instance = reasoner.is_instance_of(&ontology, &alice, &human_class).unwrap();
        assert!(is_instance, "Alice should be an instance of Person ∩ Mammal");
    }

    #[test]
    fn test_union_of_reasoning() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test".to_string());
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(1.0),
        };

        // Create union class: Student ∪ Employee
        let triples = vec![
            Triple {
                subject: "http://example.org/Student".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            Triple {
                subject: "http://example.org/Employee".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            // Define union class
            Triple {
                subject: "http://example.org/Person".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            Triple {
                subject: "http://example.org/Person".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#unionOf".to_string(),
                object: "http://example.org/PersonList".to_string(),
            },
            // List: Student OR Employee
            Triple {
                subject: "http://example.org/PersonList".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#first".to_string(),
                object: "http://example.org/Student".to_string(),
            },
            Triple {
                subject: "http://example.org/PersonList".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest".to_string(),
                object: "http://example.org/EmployeeList".to_string(),
            },
            Triple {
                subject: "http://example.org/EmployeeList".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#first".to_string(),
                object: "http://example.org/Employee".to_string(),
            },
            Triple {
                subject: "http://example.org/EmployeeList".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest".to_string(),
                object: "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil".to_string(),
            },
            // Individual who is a Student
            Triple {
                subject: "http://example.org/bob".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/bob".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://example.org/Student".to_string(),
            },
        ];

        for triple in triples {
            store.insert(triple, graph_id.clone(), provenance.clone());
        }

        let mut reasoner = OwlDlReasoner::new();
        let ontology = reasoner.load_ontology(&store).unwrap();

        // Bob should be inferred to be a Person (Student ∪ Employee)
        let bob = Individual(OwlIri::new("http://example.org/bob".to_string()));
        let person_class = ClassExpression::UnionOf(vec![
            ClassExpression::Named(OwlIri::new("http://example.org/Student".to_string())),
            ClassExpression::Named(OwlIri::new("http://example.org/Employee".to_string())),
        ]);

        let is_instance = reasoner.is_instance_of(&ontology, &bob, &person_class).unwrap();
        assert!(is_instance, "Bob should be an instance of Student ∪ Employee");
    }

    #[test]
    fn test_complement_of_reasoning() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test".to_string());
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(1.0),
        };

        // Create complement class: ¬Student
        let triples = vec![
            Triple {
                subject: "http://example.org/Student".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            Triple {
                subject: "http://example.org/Person".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            // Define complement class
            Triple {
                subject: "http://example.org/NonStudent".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            Triple {
                subject: "http://example.org/NonStudent".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#complementOf".to_string(),
                object: "http://example.org/Student".to_string(),
            },
            // Individual who is a Person but not a Student
            Triple {
                subject: "http://example.org/charlie".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/charlie".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://example.org/Person".to_string(),
            },
        ];

        for triple in triples {
            store.insert(triple, graph_id.clone(), provenance.clone());
        }

        let mut reasoner = OwlDlReasoner::new();
        let ontology = reasoner.load_ontology(&store).unwrap();

        // Charlie should be inferred to be a NonStudent (¬Student)
        let charlie = Individual(OwlIri::new("http://example.org/charlie".to_string()));
        let non_student_class = ClassExpression::ComplementOf(Box::new(
            ClassExpression::Named(OwlIri::new("http://example.org/Student".to_string()))
        ));

        let is_instance = reasoner.is_instance_of(&ontology, &charlie, &non_student_class).unwrap();
        assert!(is_instance, "Charlie should be an instance of ¬Student");
    }

    #[test]
    fn test_some_values_from_reasoning() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test".to_string());
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(1.0),
        };

        // Create someValuesFrom restriction: ∃hasChild.Person
        let triples = vec![
            Triple {
                subject: "http://example.org/Person".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            Triple {
                subject: "http://example.org/hasChild".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#ObjectProperty".to_string(),
            },
            // Define restriction class
            Triple {
                subject: "http://example.org/Parent".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Restriction".to_string(),
            },
            Triple {
                subject: "http://example.org/Parent".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#onProperty".to_string(),
                object: "http://example.org/hasChild".to_string(),
            },
            Triple {
                subject: "http://example.org/Parent".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#someValuesFrom".to_string(),
                object: "http://example.org/Person".to_string(),
            },
            // Individual with child relationship
            Triple {
                subject: "http://example.org/david".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/david".to_string(),
                predicate: "http://example.org/hasChild".to_string(),
                object: "http://example.org/emma".to_string(),
            },
            Triple {
                subject: "http://example.org/emma".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/emma".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://example.org/Person".to_string(),
            },
        ];

        for triple in triples {
            store.insert(triple, graph_id.clone(), provenance.clone());
        }

        let mut reasoner = OwlDlReasoner::new();
        let ontology = reasoner.load_ontology(&store).unwrap();

        // David should be inferred to be a Parent (∃hasChild.Person)
        let david = Individual(OwlIri::new("http://example.org/david".to_string()));
        let parent_class = ClassExpression::SomeValuesFrom {
            property: PropertyExpression::ObjectProperty(OwlIri::new("http://example.org/hasChild".to_string())),
            class: Box::new(ClassExpression::Named(OwlIri::new("http://example.org/Person".to_string()))),
        };

        let is_instance = reasoner.is_instance_of(&ontology, &david, &parent_class).unwrap();
        assert!(is_instance, "David should be an instance of ∃hasChild.Person");
    }

    #[test]
    fn test_all_values_from_reasoning() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test".to_string());
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(1.0),
        };

        // Create allValuesFrom restriction: ∀hasChild.Person
        let triples = vec![
            Triple {
                subject: "http://example.org/Person".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            Triple {
                subject: "http://example.org/hasChild".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#ObjectProperty".to_string(),
            },
            // Define restriction class
            Triple {
                subject: "http://example.org/GoodParent".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Restriction".to_string(),
            },
            Triple {
                subject: "http://example.org/GoodParent".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#onProperty".to_string(),
                object: "http://example.org/hasChild".to_string(),
            },
            Triple {
                subject: "http://example.org/GoodParent".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#allValuesFrom".to_string(),
                object: "http://example.org/Person".to_string(),
            },
            // Individual with only Person children
            Triple {
                subject: "http://example.org/eve".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/eve".to_string(),
                predicate: "http://example.org/hasChild".to_string(),
                object: "http://example.org/fiona".to_string(),
            },
            Triple {
                subject: "http://example.org/fiona".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/fiona".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://example.org/Person".to_string(),
            },
        ];

        for triple in triples {
            store.insert(triple, graph_id.clone(), provenance.clone());
        }

        let mut reasoner = OwlDlReasoner::new();
        let ontology = reasoner.load_ontology(&store).unwrap();

        // Eve should be inferred to be a GoodParent (∀hasChild.Person)
        let eve = Individual(OwlIri::new("http://example.org/eve".to_string()));
        let good_parent_class = ClassExpression::AllValuesFrom {
            property: PropertyExpression::ObjectProperty(OwlIri::new("http://example.org/hasChild".to_string())),
            class: Box::new(ClassExpression::Named(OwlIri::new("http://example.org/Person".to_string()))),
        };

        let is_instance = reasoner.is_instance_of(&ontology, &eve, &good_parent_class).unwrap();
        assert!(is_instance, "Eve should be an instance of ∀hasChild.Person");
    }

    #[test]
    fn test_has_value_reasoning() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test".to_string());
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(1.0),
        };

        // Create hasValue restriction: ∃hasChild.{john}
        let john = Individual(OwlIri::new("http://example.org/john".to_string()));
        let triples = vec![
            Triple {
                subject: "http://example.org/hasChild".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#ObjectProperty".to_string(),
            },
            // Define restriction class
            Triple {
                subject: "http://example.org/JohnsParent".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Restriction".to_string(),
            },
            Triple {
                subject: "http://example.org/JohnsParent".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#onProperty".to_string(),
                object: "http://example.org/hasChild".to_string(),
            },
            Triple {
                subject: "http://example.org/JohnsParent".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#hasValue".to_string(),
                object: "http://example.org/john".to_string(),
            },
            // Individual with john as child
            Triple {
                subject: "http://example.org/mary".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/mary".to_string(),
                predicate: "http://example.org/hasChild".to_string(),
                object: "http://example.org/john".to_string(),
            },
            Triple {
                subject: "http://example.org/john".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
        ];

        for triple in triples {
            store.insert(triple, graph_id.clone(), provenance.clone());
        }

        let mut reasoner = OwlDlReasoner::new();
        let ontology = reasoner.load_ontology(&store).unwrap();

        // Mary should be inferred to be a JohnsParent (∃hasChild.{john})
        let mary = Individual(OwlIri::new("http://example.org/mary".to_string()));
        let johns_parent_class = ClassExpression::HasValue {
            property: PropertyExpression::ObjectProperty(OwlIri::new("http://example.org/hasChild".to_string())),
            individual: fukurow_lite::model::Individual(OwlIri::new("http://example.org/john".to_string())),
        };

        let is_instance = reasoner.is_instance_of(&ontology, &mary, &johns_parent_class).unwrap();
        assert!(is_instance, "Mary should be an instance of ∃hasChild.{{specific individual}}");
    }

    #[test]
    fn test_cardinality_restrictions() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test".to_string());
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(1.0),
        };

        // Create minCardinality restriction: ≥2 hasChild
        let triples = vec![
            Triple {
                subject: "http://example.org/hasChild".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#ObjectProperty".to_string(),
            },
            // Define restriction class
            Triple {
                subject: "http://example.org/MultiChildParent".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Restriction".to_string(),
            },
            Triple {
                subject: "http://example.org/MultiChildParent".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#onProperty".to_string(),
                object: "http://example.org/hasChild".to_string(),
            },
            Triple {
                subject: "http://example.org/MultiChildParent".to_string(),
                predicate: "http://www.w3.org/2002/07/owl#minCardinality".to_string(),
                object: "2".to_string(),
            },
            // Individual with 2 children
            Triple {
                subject: "http://example.org/george".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/george".to_string(),
                predicate: "http://example.org/hasChild".to_string(),
                object: "http://example.org/harry".to_string(),
            },
            Triple {
                subject: "http://example.org/george".to_string(),
                predicate: "http://example.org/hasChild".to_string(),
                object: "http://example.org/ian".to_string(),
            },
            Triple {
                subject: "http://example.org/harry".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
            Triple {
                subject: "http://example.org/ian".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            },
        ];

        for triple in triples {
            store.insert(triple, graph_id.clone(), provenance.clone());
        }

        let mut reasoner = OwlDlReasoner::new();
        let ontology = reasoner.load_ontology(&store).unwrap();

        // George should be inferred to be a MultiChildParent (≥2 hasChild)
        let george = Individual(OwlIri::new("http://example.org/george".to_string()));
        let multi_child_parent_class = ClassExpression::MinCardinality {
            cardinality: 2,
            property: PropertyExpression::ObjectProperty(OwlIri::new("http://example.org/hasChild".to_string())),
            class: None, // None means owl:Thing
        };

        let is_instance = reasoner.is_instance_of(&ontology, &george, &multi_child_parent_class).unwrap();
        assert!(is_instance, "George should be an instance of ≥2 hasChild");
    }
}
