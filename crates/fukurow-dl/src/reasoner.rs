//! OWL DL リーナー

use crate::model::{OwlDlOntology, ClassExpression, PropertyExpression, Axiom};
use crate::loader::OwlDlOntologyLoader;
use crate::tableau::DlTableauReasoner;
use crate::OwlDlError;
use fukurow_store::store::RdfStore;
use fukurow_lite::{OwlLiteReasoner, Ontology as OwlLiteOntology};
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
        // For now, fall back to OWL Lite reasoning for simple cases
        match class_expr {
            ClassExpression::Named(iri) => {
                let lite_class = fukurow_lite::Class::Named(iri.clone());
                self.lite_reasoner.is_instance_of(&OwlLiteOntology::new(), individual, &lite_class)
                    .map_err(|e| OwlDlError::ReasoningError(e.to_string()))
            }
            ClassExpression::Thing => Ok(true),
            ClassExpression::Nothing => Ok(false),
            _ => Err(OwlDlError::UnsupportedFeature("Complex instance checking not yet implemented".to_string()))
        }
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
}
