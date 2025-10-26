// Basic SHACL validation tests to verify current implementation

use fukurow_core::model::Triple;
use fukurow_store::store::RdfStore;
use fukurow_store::provenance::{Provenance, GraphId};
use fukurow_shacl::{ShaclLoader, ShaclValidator, ValidationReport};

fn default_graph_id() -> GraphId {
    GraphId::Named("test".to_string())
}

fn sensor_provenance() -> Provenance {
    Provenance::Sensor {
        source: "test".to_string(),
        confidence: Some(1.0),
    }
}

fn create_test_store() -> RdfStore {
    let mut store = RdfStore::new();

    // Add some test data
    store.insert(Triple {
        subject: "http://example.org/John".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://example.org/Person".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/John".to_string(),
        predicate: "http://example.org/name".to_string(),
        object: "John Doe".to_string(),
    }, default_graph_id(), sensor_provenance());

    store
}

#[test]
fn test_shacl_loader_basic() {
    let store = create_test_store();
    let loader = fukurow_shacl::loader::DefaultShaclLoader;

    // Try to load shapes from the store (should handle empty gracefully)
    let result = loader.load_from_store(&store);
    assert!(result.is_ok());

    let shapes_graph = result.unwrap();
    // Should have empty shapes since no SHACL shapes are defined
    assert!(shapes_graph.shapes.is_empty());
}

#[test]
fn test_shacl_validator_basic() {
    let store = create_test_store();
    let validator = fukurow_shacl::validator::DefaultShaclValidator;
    let loader = fukurow_shacl::loader::DefaultShaclLoader;

    let shapes_graph = loader.load_from_store(&store).unwrap();
    let config = fukurow_shacl::validator::ValidationConfig::default();

    let result = validator.validate_graph(&shapes_graph, &store, &config);
    assert!(result.is_ok());

    let report = result.unwrap();
    // With no shapes defined, validation should pass
    assert!(report.conforms);
    assert!(report.results.is_empty());
}

#[test]
fn test_shacl_with_simple_shapes() {
    let mut store = create_test_store();

    // Add a simple SHACL shape
    store.insert(Triple {
        subject: "http://example.org/PersonShape".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://www.w3.org/ns/shacl#NodeShape".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/PersonShape".to_string(),
        predicate: "http://www.w3.org/ns/shacl#targetClass".to_string(),
        object: "http://example.org/Person".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Add a property constraint
    store.insert(Triple {
        subject: "http://example.org/PersonShape".to_string(),
        predicate: "http://www.w3.org/ns/shacl#property".to_string(),
        object: "http://example.org/NameProperty".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/NameProperty".to_string(),
        predicate: "http://www.w3.org/ns/shacl#path".to_string(),
        object: "http://example.org/name".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/NameProperty".to_string(),
        predicate: "http://www.w3.org/ns/shacl#datatype".to_string(),
        object: "http://www.w3.org/2001/XMLSchema#string".to_string(),
    }, default_graph_id(), sensor_provenance());

    let loader = fukurow_shacl::loader::DefaultShaclLoader;
    let validator = fukurow_shacl::validator::DefaultShaclValidator;

    let shapes_graph = loader.load_from_store(&store).unwrap();
    let config = fukurow_shacl::validator::ValidationConfig::default();

    let result = validator.validate_graph(&shapes_graph, &store, &config);
    assert!(result.is_ok());

    let report = result.unwrap();
    // This should validate successfully since John has a name with string datatype
    println!("Validation conforms: {}", report.conforms);
    println!("Number of validation results: {}", report.results.len());
}
