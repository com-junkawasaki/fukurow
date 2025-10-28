//! OWL Lite 推論エンジン
//!
//! このクレートは OWL Lite の完全実装を提供します:
//! - テーブルロー推論アルゴリズム
//! - 整合性検証
//! - クラス階層推論
//! - インスタンス検証

pub mod model;
pub mod tableau;
pub mod reasoner;
pub mod loader;

pub use model::{Ontology, Class, Property, Individual, Axiom};
pub use reasoner::OwlLiteReasoner;
pub use loader::OntologyLoader;

// Re-export store types for WASM integration
pub use fukurow_store::store::RdfStore;
pub use fukurow_store::provenance::{Provenance, GraphId};
pub use fukurow_core::model::Triple;

// Error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OwlError {
    #[error("Loader error: {0}")]
    LoaderError(String),

    #[error("Reasoning error: {0}")]
    ReasoningError(String),

    #[error("Consistency error: {0}")]
    ConsistencyError(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}

#[cfg(test)]
fn load_ontology_from_triples(triples: Vec<fukurow_core::model::Triple>) -> Result<crate::model::Ontology, OwlError> {
    let mut store = fukurow_store::store::RdfStore::new();
    for triple in triples {
        store.insert(triple, fukurow_store::provenance::GraphId::Default, fukurow_store::provenance::Provenance::Sensor {
            source: "test".to_string(),
            confidence: None,
        });
    }
    let loader = crate::loader::DefaultOntologyLoader;
    loader.load_from_store(&store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_core::model::Triple;

    mod loader_tests {
        use super::*;
        use crate::model::{Ontology, Class, Property, Individual, Axiom, OwlIri};

        fn create_test_triples() -> Vec<Triple> {
            vec![
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
                // Property declarations
                Triple {
                    subject: "http://example.org/hasParent".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#ObjectProperty".to_string(),
                },
                Triple {
                    subject: "http://example.org/hasAge".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#DatatypeProperty".to_string(),
                },
                // Individuals
                Triple {
                    subject: "http://example.org/john".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
                },
                Triple {
                    subject: "http://example.org/mary".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
                },
                // Class assertions
                Triple {
                    subject: "http://example.org/john".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://example.org/Person".to_string(),
                },
                Triple {
                    subject: "http://example.org/mary".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://example.org/Person".to_string(),
                },
                // Subclass relation
                Triple {
                    subject: "http://example.org/Student".to_string(),
                    predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                    object: "http://example.org/Person".to_string(),
                },
            ]
        }


        #[test]
        fn test_load_empty_ontology() {
            let triples = Vec::new();
            let ontology = load_ontology_from_triples(triples).unwrap();

            assert!(ontology.classes.is_empty());
            assert!(ontology.properties.is_empty());
            assert!(ontology.individuals.is_empty());
            assert!(ontology.axioms.is_empty());
        }

        #[test]
        fn test_load_classes() {
            let triples = vec![
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
            ];

            let ontology = load_ontology_from_triples(triples).unwrap();

            assert_eq!(ontology.classes.len(), 2);
            assert!(ontology.classes.contains(&Class::Named(OwlIri::new("http://example.org/Person".to_string()))));
            assert!(ontology.classes.contains(&Class::Named(OwlIri::new("http://example.org/Animal".to_string()))));
        }

        #[test]
        fn test_load_properties() {
            let triples = vec![
                Triple {
                    subject: "http://example.org/hasParent".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#ObjectProperty".to_string(),
                },
                Triple {
                    subject: "http://example.org/hasAge".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#DatatypeProperty".to_string(),
                },
            ];

            let ontology = load_ontology_from_triples(triples).unwrap();

            assert_eq!(ontology.properties.len(), 2);
            assert!(ontology.properties.contains(&Property::Object(OwlIri::new("http://example.org/hasParent".to_string()))));
            assert!(ontology.properties.contains(&Property::Data(OwlIri::new("http://example.org/hasAge".to_string()))));
        }

        #[test]
        fn test_load_individuals() {
            let triples = vec![
                Triple {
                    subject: "http://example.org/john".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
                },
                Triple {
                    subject: "http://example.org/mary".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
                },
            ];

            let ontology = load_ontology_from_triples(triples).unwrap();

            assert_eq!(ontology.individuals.len(), 2);
            assert!(ontology.individuals.contains(&Individual(OwlIri::new("http://example.org/john".to_string()))));
            assert!(ontology.individuals.contains(&Individual(OwlIri::new("http://example.org/mary".to_string()))));
        }

        #[test]
        fn test_load_class_assertions() {
            let triples = vec![
                // Named individual
                Triple {
                    subject: "http://example.org/john".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
                },
                // Class
                Triple {
                    subject: "http://example.org/Person".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#Class".to_string(),
                },
                // Class assertion
                Triple {
                    subject: "http://example.org/john".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://example.org/Person".to_string(),
                },
            ];

            let ontology = load_ontology_from_triples(triples).unwrap();

            assert_eq!(ontology.classes.len(), 1);
            assert_eq!(ontology.individuals.len(), 1);
            assert_eq!(ontology.axioms.len(), 1);

            // Check for class assertion axiom
            match &ontology.axioms[0] {
                Axiom::ClassAssertion(class, individual) => {
                    assert_eq!(*class, Class::Named(OwlIri::new("http://example.org/Person".to_string())));
                    assert_eq!(*individual, Individual(OwlIri::new("http://example.org/john".to_string())));
                }
                _ => panic!("Expected ClassAssertion axiom"),
            }
        }

        #[test]
        fn test_load_subclass_relation() {
            let triples = vec![
                // Classes
                Triple {
                    subject: "http://example.org/Person".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#Class".to_string(),
                },
                Triple {
                    subject: "http://example.org/Student".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#Class".to_string(),
                },
                // Subclass relation
                Triple {
                    subject: "http://example.org/Student".to_string(),
                    predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                    object: "http://example.org/Person".to_string(),
                },
            ];

            let ontology = load_ontology_from_triples(triples).unwrap();

            assert_eq!(ontology.classes.len(), 2);
            assert_eq!(ontology.axioms.len(), 1);

            // Check for subclass axiom
            match &ontology.axioms[0] {
                Axiom::SubClassOf(subclass, superclass) => {
                    assert_eq!(*subclass, Class::Named(OwlIri::new("http://example.org/Student".to_string())));
                    assert_eq!(*superclass, Class::Named(OwlIri::new("http://example.org/Person".to_string())));
                }
                _ => panic!("Expected SubClassOf axiom"),
            }
        }

        #[test]
        fn test_load_complex_ontology() {
            let triples = create_test_triples();

            let ontology = load_ontology_from_triples(triples).unwrap();

            assert_eq!(ontology.classes.len(), 3); // Person, Animal, Student
            assert_eq!(ontology.properties.len(), 2); // hasParent, hasAge
            assert_eq!(ontology.individuals.len(), 2); // john, mary
            assert_eq!(ontology.axioms.len(), 3); // 2 class assertions + 1 subclass
        }
    }

    mod reasoner_tests {
        use super::*;
        use crate::reasoner::OwlLiteReasoner;
        use crate::model::{Class, Individual, OwlIri};

        #[test]
        fn test_reasoner_creation() {
            let reasoner = OwlLiteReasoner::new();
            // Basic functionality test
            assert!(true); // Just ensure it can be created
        }

        #[test]
        fn test_consistency_check() {
            let reasoner = OwlLiteReasoner::new();

            // Create a simple consistent ontology
            let triples = vec![
                Triple {
                    subject: "http://example.org/Person".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#Class".to_string(),
                },
                Triple {
                    subject: "http://example.org/john".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
                },
                Triple {
                    subject: "http://example.org/john".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://example.org/Person".to_string(),
                },
            ];

            let ontology = load_ontology_from_triples(triples).unwrap();
            let mut reasoner = OwlLiteReasoner::new();
            let result = reasoner.is_consistent(&ontology);

            // For now, assume consistency check passes for simple ontologies
            assert!(result.unwrap_or(false));
        }

        #[test]
        fn test_class_hierarchy_inference() {
            let reasoner = OwlLiteReasoner::new();

            let triples = vec![
                // Classes
                Triple {
                    subject: "http://example.org/Person".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#Class".to_string(),
                },
                Triple {
                    subject: "http://example.org/Student".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#Class".to_string(),
                },
                // Subclass
                Triple {
                    subject: "http://example.org/Student".to_string(),
                    predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                    object: "http://example.org/Person".to_string(),
                },
                // Individual
                Triple {
                    subject: "http://example.org/john".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
                },
                // Instance of Student
                Triple {
                    subject: "http://example.org/john".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://example.org/Student".to_string(),
                },
            ];

            let ontology = load_ontology_from_triples(triples).unwrap();
            let mut reasoner = OwlLiteReasoner::new();
            let inferred = reasoner.get_inferred_axioms(&ontology).unwrap();

            // Check that john is also inferred to be a Person
            let john_person_assertion = inferred.iter().any(|axiom| {
                matches!(axiom, Axiom::ClassAssertion(class, individual)
                    if *class == Class::Named(OwlIri::new("http://example.org/Person".to_string()))
                    && *individual == Individual(OwlIri::new("http://example.org/john".to_string())))
            });

            assert!(john_person_assertion, "john should be inferred to be a Person");
        }
    }

    mod model_tests {
        use super::*;
        use crate::model::{Ontology, Class, Property, Individual, OwlIri};

        #[test]
        fn test_ontology_creation() {
            let ontology = Ontology::new();
            assert!(ontology.classes.is_empty());
            assert!(ontology.properties.is_empty());
            assert!(ontology.individuals.is_empty());
            assert!(ontology.axioms.is_empty());
        }

        #[test]
        fn test_class_equality() {
            let class1 = Class::Named(OwlIri::new("http://example.org/Person".to_string()));
            let class2 = Class::Named(OwlIri::new("http://example.org/Person".to_string()));
            let class3 = Class::Named(OwlIri::new("http://example.org/Animal".to_string()));

            assert_eq!(class1, class2);
            assert_ne!(class1, class3);
        }

        #[test]
        fn test_individual_equality() {
            let ind1 = Individual(OwlIri::new("http://example.org/john".to_string()));
            let ind2 = Individual(OwlIri::new("http://example.org/john".to_string()));
            let ind3 = Individual(OwlIri::new("http://example.org/mary".to_string()));

            assert_eq!(ind1, ind2);
            assert_ne!(ind1, ind3);
        }

        #[test]
        fn test_property_equality() {
            let prop1 = Property::Object(OwlIri::new("http://example.org/hasParent".to_string()));
            let prop2 = Property::Object(OwlIri::new("http://example.org/hasParent".to_string()));
            let prop3 = Property::Data(OwlIri::new("http://example.org/hasAge".to_string()));

            assert_eq!(prop1, prop2);
            assert_ne!(prop1, prop3);
        }

        #[test]
        fn test_owl_iri_creation() {
            let iri = OwlIri::new("http://example.org/Person".to_string());
            assert_eq!(iri.0, "http://example.org/Person");
        }
    }
}
