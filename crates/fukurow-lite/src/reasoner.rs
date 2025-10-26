//! OWL Lite リーナー

use crate::model::{Ontology, Class, Property, Individual, Axiom, OwlIri};
use crate::loader::{OntologyLoader, DefaultOntologyLoader};
use crate::tableau::TableauReasoner;
use crate::OwlError;
use fukurow_store::store::RdfStore;
use std::collections::{HashMap, HashSet};

/// OWL Lite reasoner
pub struct OwlLiteReasoner {
    loader: DefaultOntologyLoader,
    tableau: TableauReasoner,
}

impl OwlLiteReasoner {
    pub fn new() -> Self {
        Self {
            loader: DefaultOntologyLoader,
            tableau: TableauReasoner::new(),
        }
    }

    /// Load ontology from RDF store
    pub fn load_ontology(&self, store: &RdfStore) -> Result<Ontology, OwlError> {
        self.loader.load_from_store(store)
    }

    /// Check if ontology is consistent
    pub fn is_consistent(&mut self, ontology: &Ontology) -> Result<bool, OwlError> {
        self.tableau.is_consistent(ontology)
    }

    /// Compute class subsumption hierarchy
    pub fn compute_class_hierarchy(&mut self, ontology: &Ontology) -> Result<HashMap<Class, HashSet<Class>>, OwlError> {
        self.tableau.compute_subsumption_hierarchy(ontology)
    }

    /// Check if class C1 is subsumed by class C2 (C1 ⊑ C2)
    pub fn is_subsumed_by(&mut self, ontology: &Ontology, subclass: &Class, superclass: &Class) -> Result<bool, OwlError> {
        let hierarchy = self.compute_class_hierarchy(ontology)?;

        if let Some(supers) = hierarchy.get(subclass) {
            Ok(supers.contains(superclass))
        } else {
            Ok(false)
        }
    }

    /// Get all subclasses of a given class
    pub fn get_subclasses(&mut self, ontology: &Ontology, class: &Class) -> Result<HashSet<Class>, OwlError> {
        let hierarchy = self.compute_class_hierarchy(ontology)?;
        let mut subclasses = HashSet::new();

        // Find all classes that have the given class as a superclass
        for (subclass, supers) in &hierarchy {
            if supers.contains(class) {
                subclasses.insert(subclass.clone());
            }
        }

        // Include the class itself
        subclasses.insert(class.clone());

        Ok(subclasses)
    }

    /// Get all superclasses of a given class
    pub fn get_superclasses(&mut self, ontology: &Ontology, class: &Class) -> Result<HashSet<Class>, OwlError> {
        let hierarchy = self.compute_class_hierarchy(ontology)?;

        let mut superclasses = HashSet::new();

        // Start with direct superclasses
        if let Some(direct_supers) = hierarchy.get(class) {
            superclasses.extend(direct_supers.iter().cloned());
        }

        // Include the class itself
        superclasses.insert(class.clone());

        Ok(superclasses)
    }

    /// Check if individual is instance of class
    pub fn is_instance_of(&mut self, _ontology: &Ontology, _individual: &Individual, _class: &Class) -> Result<bool, OwlError> {
        // This would require instance checking, which is more complex
        // For now, return true if there's a ClassAssertion axiom
        // TODO: Implement proper instance checking with tableau expansion
        Err(OwlError::UnsupportedFeature("Instance checking not yet implemented".to_string()))
    }

    /// Classify ontology (compute complete class hierarchy)
    pub fn classify_ontology(&mut self, ontology: &Ontology) -> Result<HashMap<Class, HashSet<Class>>, OwlError> {
        self.compute_class_hierarchy(ontology)
    }

    /// Realize ontology (compute individual types)
    pub fn realize_ontology(&mut self, _ontology: &Ontology) -> Result<HashMap<Individual, HashSet<Class>>, OwlError> {
        // TODO: Implement realization (instance classification)
        Err(OwlError::UnsupportedFeature("Ontology realization not yet implemented".to_string()))
    }

    /// Get inferred axioms (closure of the ontology)
    pub fn get_inferred_axioms(&mut self, ontology: &Ontology) -> Result<Vec<Axiom>, OwlError> {
        let hierarchy = self.compute_class_hierarchy(ontology)?;
        let mut inferred = Vec::new();

        // Generate SubClassOf axioms from hierarchy
        for (subclass, superclasses) in &hierarchy {
            for superclass in superclasses {
                if subclass != superclass { // Avoid self-subsumption
                    inferred.push(Axiom::SubClassOf(subclass.clone(), superclass.clone()));
                }
            }
        }

        // TODO: Add other inferred axioms (property hierarchies, etc.)

        Ok(inferred)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_store::store::RdfStore;
    use fukurow_store::provenance::{Provenance, GraphId};
    use fukurow_core::model::Triple;
    use chrono::Utc;

    fn create_test_store() -> RdfStore {
        let mut store = RdfStore::new();

        // Create a simple ontology: Person ⊑ Animal, Student ⊑ Person
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
            Triple {
                subject: "http://example.org/Student".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            },
            // Subclass relations
            Triple {
                subject: "http://example.org/Person".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                object: "http://example.org/Animal".to_string(),
            },
            Triple {
                subject: "http://example.org/Student".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                object: "http://example.org/Person".to_string(),
            },
        ];

        let graph_id = GraphId::Named("test".to_string());
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(1.0),
        };

        for triple in triples {
            store.insert(triple, graph_id.clone(), provenance.clone());
        }

        store
    }

    #[test]
    fn test_load_ontology() {
        let store = create_test_store();
        let reasoner = OwlLiteReasoner::new();

        let ontology = reasoner.load_ontology(&store).unwrap();

        assert!(ontology.classes.contains(&Class::Named(OwlIri::new("http://example.org/Person".to_string()))));
        assert!(ontology.classes.contains(&Class::Named(OwlIri::new("http://example.org/Animal".to_string()))));
        assert!(ontology.classes.contains(&Class::Named(OwlIri::new("http://example.org/Student".to_string()))));

        // Check for SubClassOf axioms
        let person_animal = ontology.axioms.iter().any(|axiom| {
            matches!(axiom, Axiom::SubClassOf(Class::Named(p), Class::Named(a))
                if p.0 == "http://example.org/Person" && a.0 == "http://example.org/Animal")
        });
        assert!(person_animal);

        let student_person = ontology.axioms.iter().any(|axiom| {
            matches!(axiom, Axiom::SubClassOf(Class::Named(s), Class::Named(p))
                if s.0 == "http://example.org/Student" && p.0 == "http://example.org/Person")
        });
        assert!(student_person);
    }

    #[test]
    fn test_consistency_check() {
        let store = create_test_store();
        let mut reasoner = OwlLiteReasoner::new();

        let ontology = reasoner.load_ontology(&store).unwrap();
        let is_consistent = reasoner.is_consistent(&ontology).unwrap();

        assert!(is_consistent);
    }

    #[test]
    fn test_class_hierarchy() {
        let store = create_test_store();
        let mut reasoner = OwlLiteReasoner::new();

        let ontology = reasoner.load_ontology(&store).unwrap();
        let hierarchy = reasoner.compute_class_hierarchy(&ontology).unwrap();

        let student = Class::Named(OwlIri::new("http://example.org/Student".to_string()));
        let person = Class::Named(OwlIri::new("http://example.org/Person".to_string()));
        let animal = Class::Named(OwlIri::new("http://example.org/Animal".to_string()));

        // Student should be subsumed by Person
        assert!(hierarchy.get(&student).unwrap().contains(&person));

        // Student should be subsumed by Animal (transitive)
        assert!(hierarchy.get(&student).unwrap().contains(&animal));

        // Person should be subsumed by Animal
        assert!(hierarchy.get(&person).unwrap().contains(&animal));
    }
}
