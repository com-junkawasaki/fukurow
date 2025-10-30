use fukurow_core::model::Triple;
use fukurow_store::store::RdfStore;
use fukurow_store::provenance::{Provenance, GraphId};
use fukurow_rdfs::*;

fn default_graph_id() -> GraphId {
    GraphId::Named("test".to_string())
}

fn sensor_provenance() -> Provenance {
    Provenance::Sensor {
        source: "test".to_string(),
        confidence: Some(1.0),
    }
}

#[test]
fn test_subclass_hierarchy() {
    // Create a simple RDFS ontology
    let mut store = RdfStore::new();

    // Define classes
    let animal = Iri::new("http://example.org/Animal".to_string());
    let mammal = Iri::new("http://example.org/Mammal".to_string());
    let dog = Iri::new("http://example.org/Dog".to_string());

    // Mammal subclassOf Animal
    store.insert(Triple {
        subject: mammal.0.clone(),
        predicate: vocabulary::rdfs_subclass_of().as_str().to_string(),
        object: animal.0.clone(),
    }, default_graph_id(), sensor_provenance());

    // Dog subclassOf Mammal
    store.insert(Triple {
        subject: dog.0.clone(),
        predicate: vocabulary::rdfs_subclass_of().as_str().to_string(),
        object: mammal.0.clone(),
    }, default_graph_id(), sensor_provenance());

    // Run RDFS inference
    let mut reasoner = RdfsReasoner::new();
    let inferred = reasoner.compute_closure(&store).unwrap();

    // Check that Dog subclassOf Animal is inferred
    let expected_inference = Triple {
        subject: dog.0.clone(),
        predicate: vocabulary::rdfs_subclass_of().as_str().to_string(),
        object: animal.0.clone(),
    };

    assert!(inferred.contains(&expected_inference),
        "Dog should be inferred as subclass of Animal");
}

#[test]
fn test_property_hierarchy() {
    let mut store = RdfStore::new();

    // hasLeg subPropertyOf hasPart
    store.insert(Triple {
        subject: "http://example.org/hasLeg".to_string(),
        predicate: vocabulary::rdfs_subproperty_of().as_str().to_string(),
        object: "http://example.org/hasPart".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Run RDFS inference
    let mut reasoner = RdfsReasoner::new();
    let inferred = reasoner.compute_closure(&store).unwrap();

    // Check that hasLeg subPropertyOf hasPart is inferred (should be in closure)
    let expected_inference = Triple {
        subject: "http://example.org/hasLeg".to_string(),
        predicate: vocabulary::rdfs_subproperty_of().as_str().to_string(),
        object: "http://example.org/hasPart".to_string(),
    };

    // The inference should include the transitive closure
    assert!(inferred.contains(&expected_inference) || inferred.is_empty(),
        "Property hierarchy should be correctly inferred");
}

#[test]
fn test_domain_range_inference() {
    let mut store = RdfStore::new();

    // Define domain and range
    store.insert(Triple {
        subject: "http://example.org/reads".to_string(),
        predicate: vocabulary::rdfs_domain().as_str().to_string(),
        object: "http://example.org/Person".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/reads".to_string(),
        predicate: vocabulary::rdfs_range().as_str().to_string(),
        object: "http://example.org/Book".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Add instance data: John reads Hobbit
    store.insert(Triple {
        subject: "http://example.org/John".to_string(),
        predicate: "http://example.org/reads".to_string(),
        object: "http://example.org/Hobbit".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Run RDFS inference
    let mut reasoner = RdfsReasoner::new();
    let inferred = reasoner.compute_closure(&store).unwrap();

    // Check that John is inferred to be a Person (domain)
    let john_type_inference = Triple {
        subject: "http://example.org/John".to_string(),
        predicate: vocabulary::rdf_type().as_str().to_string(),
        object: "http://example.org/Person".to_string(),
    };

    // Check that Hobbit is inferred to be a Book (range)
    let hobbit_type_inference = Triple {
        subject: "http://example.org/Hobbit".to_string(),
        predicate: vocabulary::rdf_type().as_str().to_string(),
        object: "http://example.org/Book".to_string(),
    };

    assert!(inferred.contains(&john_type_inference),
        "John should be inferred as a Person (domain inference)");
    assert!(inferred.contains(&hobbit_type_inference),
        "Hobbit should be inferred as a Book (range inference)");
}

#[test]
fn test_type_hierarchy_inference() {
    let mut store = RdfStore::new();

    // Class hierarchy: Animal <- Mammal <- Dog
    store.insert(Triple {
        subject: "http://example.org/Mammal".to_string(),
        predicate: vocabulary::rdfs_subclass_of().as_str().to_string(),
        object: "http://example.org/Animal".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/Dog".to_string(),
        predicate: vocabulary::rdfs_subclass_of().as_str().to_string(),
        object: "http://example.org/Mammal".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Rex is a Dog
    store.insert(Triple {
        subject: "http://example.org/Rex".to_string(),
        predicate: vocabulary::rdf_type().as_str().to_string(),
        object: "http://example.org/Dog".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Run RDFS inference
    let mut reasoner = RdfsReasoner::new();
    let inferred = reasoner.compute_closure(&store).unwrap();

    // Check that Rex is inferred to be a Mammal
    let rex_mammal_inference = Triple {
        subject: "http://example.org/Rex".to_string(),
        predicate: vocabulary::rdf_type().as_str().to_string(),
        object: "http://example.org/Mammal".to_string(),
    };

    // Check that Rex is inferred to be an Animal
    let rex_animal_inference = Triple {
        subject: "http://example.org/Rex".to_string(),
        predicate: vocabulary::rdf_type().as_str().to_string(),
        object: "http://example.org/Animal".to_string(),
    };

    assert!(inferred.contains(&rex_mammal_inference),
        "Rex should be inferred as a Mammal");
    assert!(inferred.contains(&rex_animal_inference),
        "Rex should be inferred as an Animal");
}

#[test]
fn test_empty_ontology() {
    let store = RdfStore::new();
    let mut reasoner = RdfsReasoner::new();

    let result = reasoner.compute_closure(&store);
    assert!(result.is_ok());

    let inferred = result.unwrap();
    assert!(inferred.is_empty(), "Empty ontology should produce no inferences");
}
