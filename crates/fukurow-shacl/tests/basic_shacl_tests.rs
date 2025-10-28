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

#[test]
fn test_shacl_datatype_constraint() {
    let mut store = RdfStore::new();

    // Add test data with wrong datatype
    store.insert(Triple {
        subject: "http://example.org/John".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://example.org/Person".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/John".to_string(),
        predicate: "http://example.org/age".to_string(),
        object: "25".to_string(), // Should be integer, but stored as string
    }, default_graph_id(), sensor_provenance());

    // Add SHACL shape requiring integer datatype
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

    store.insert(Triple {
        subject: "http://example.org/PersonShape".to_string(),
        predicate: "http://www.w3.org/ns/shacl#property".to_string(),
        object: "http://example.org/AgeProperty".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/AgeProperty".to_string(),
        predicate: "http://www.w3.org/ns/shacl#path".to_string(),
        object: "http://example.org/age".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/AgeProperty".to_string(),
        predicate: "http://www.w3.org/ns/shacl#datatype".to_string(),
        object: "http://www.w3.org/2001/XMLSchema#integer".to_string(),
    }, default_graph_id(), sensor_provenance());

    let loader = fukurow_shacl::loader::DefaultShaclLoader;
    let validator = fukurow_shacl::validator::DefaultShaclValidator;

    let shapes_graph = loader.load_from_store(&store).unwrap();
    let config = fukurow_shacl::validator::ValidationConfig::default();

    let result = validator.validate_graph(&shapes_graph, &store, &config);
    assert!(result.is_ok());

    let report = result.unwrap();
    // Should not conform due to datatype mismatch
    assert!(!report.conforms);
    assert!(!report.results.is_empty());
}

#[test]
fn test_shacl_min_count_constraint() {
    let mut store = RdfStore::new();

    // Add person without required name property
    store.insert(Triple {
        subject: "http://example.org/Jane".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://example.org/Person".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Add SHACL shape requiring name property
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
        predicate: "http://www.w3.org/ns/shacl#minCount".to_string(),
        object: "1".to_string(),
    }, default_graph_id(), sensor_provenance());

    let loader = fukurow_shacl::loader::DefaultShaclLoader;
    let validator = fukurow_shacl::validator::DefaultShaclValidator;

    let shapes_graph = loader.load_from_store(&store).unwrap();
    let config = fukurow_shacl::validator::ValidationConfig::default();

    let result = validator.validate_graph(&shapes_graph, &store, &config);
    assert!(result.is_ok());

    let report = result.unwrap();
    // Should not conform due to missing required property
    assert!(!report.conforms);
    assert!(!report.results.is_empty());
}

#[test]
fn test_shacl_max_count_constraint() {
    let mut store = RdfStore::new();

    // Add person with multiple names (violating maxCount=1)
    store.insert(Triple {
        subject: "http://example.org/Bob".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://example.org/Person".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/Bob".to_string(),
        predicate: "http://example.org/name".to_string(),
        object: "Bob Smith".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/Bob".to_string(),
        predicate: "http://example.org/name".to_string(),
        object: "Robert Smith".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Add SHACL shape with maxCount=1
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
        predicate: "http://www.w3.org/ns/shacl#maxCount".to_string(),
        object: "1".to_string(),
    }, default_graph_id(), sensor_provenance());

    let loader = fukurow_shacl::loader::DefaultShaclLoader;
    let validator = fukurow_shacl::validator::DefaultShaclValidator;

    let shapes_graph = loader.load_from_store(&store).unwrap();
    let config = fukurow_shacl::validator::ValidationConfig::default();

    let result = validator.validate_graph(&shapes_graph, &store, &config);
    assert!(result.is_ok());

    let report = result.unwrap();
    // Should not conform due to too many values
    assert!(!report.conforms);
    assert!(!report.results.is_empty());
}

#[test]
fn test_shacl_has_value_constraint() {
    let mut store = RdfStore::new();

    // Add person without required value
    store.insert(Triple {
        subject: "http://example.org/Alice".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://example.org/Student".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Add SHACL shape requiring specific value
    store.insert(Triple {
        subject: "http://example.org/StudentShape".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://www.w3.org/ns/shacl#NodeShape".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/StudentShape".to_string(),
        predicate: "http://www.w3.org/ns/shacl#targetClass".to_string(),
        object: "http://example.org/Student".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/StudentShape".to_string(),
        predicate: "http://www.w3.org/ns/shacl#property".to_string(),
        object: "http://example.org/TypeProperty".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/TypeProperty".to_string(),
        predicate: "http://www.w3.org/ns/shacl#path".to_string(),
        object: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/TypeProperty".to_string(),
        predicate: "http://www.w3.org/ns/shacl#hasValue".to_string(),
        object: "http://example.org/Student".to_string(),
    }, default_graph_id(), sensor_provenance());

    let loader = fukurow_shacl::loader::DefaultShaclLoader;
    let validator = fukurow_shacl::validator::DefaultShaclValidator;

    let shapes_graph = loader.load_from_store(&store).unwrap();
    let config = fukurow_shacl::validator::ValidationConfig::default();

    let result = validator.validate_graph(&shapes_graph, &store, &config);
    assert!(result.is_ok());

    let report = result.unwrap();
    // Should conform since Alice has the required rdf:type value
    assert!(report.conforms);
}

#[test]
fn test_shacl_class_constraint() {
    let mut store = RdfStore::new();

    // Add person with invalid class membership
    store.insert(Triple {
        subject: "http://example.org/Tom".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://example.org/Person".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/Tom".to_string(),
        predicate: "http://example.org/manager".to_string(),
        object: "http://example.org/InvalidClass".to_string(), // Not a valid class
    }, default_graph_id(), sensor_provenance());

    // Add SHACL shape with class constraint
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

    store.insert(Triple {
        subject: "http://example.org/PersonShape".to_string(),
        predicate: "http://www.w3.org/ns/shacl#property".to_string(),
        object: "http://example.org/ManagerProperty".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/ManagerProperty".to_string(),
        predicate: "http://www.w3.org/ns/shacl#path".to_string(),
        object: "http://example.org/manager".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/ManagerProperty".to_string(),
        predicate: "http://www.w3.org/ns/shacl#class".to_string(),
        object: "http://example.org/Person".to_string(),
    }, default_graph_id(), sensor_provenance());

    let loader = fukurow_shacl::loader::DefaultShaclLoader;
    let validator = fukurow_shacl::validator::DefaultShaclValidator;

    let shapes_graph = loader.load_from_store(&store).unwrap();
    let config = fukurow_shacl::validator::ValidationConfig::default();

    let result = validator.validate_graph(&shapes_graph, &store, &config);
    assert!(result.is_ok());

    let report = result.unwrap();
    // Should not conform due to invalid class membership
    assert!(!report.conforms);
    assert!(!report.results.is_empty());
}

#[test]
fn test_shacl_multiple_shapes() {
    let mut store = RdfStore::new();

    // Add data for multiple shapes
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

    store.insert(Triple {
        subject: "http://example.org/Company".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://example.org/Organization".to_string(),
    }, default_graph_id(), sensor_provenance());

    // Add multiple SHACL shapes
    // Person shape
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

    // Organization shape
    store.insert(Triple {
        subject: "http://example.org/OrgShape".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://www.w3.org/ns/shacl#NodeShape".to_string(),
    }, default_graph_id(), sensor_provenance());

    store.insert(Triple {
        subject: "http://example.org/OrgShape".to_string(),
        predicate: "http://www.w3.org/ns/shacl#targetClass".to_string(),
        object: "http://example.org/Organization".to_string(),
    }, default_graph_id(), sensor_provenance());

    let loader = fukurow_shacl::loader::DefaultShaclLoader;
    let validator = fukurow_shacl::validator::DefaultShaclValidator;

    let shapes_graph = loader.load_from_store(&store).unwrap();
    let config = fukurow_shacl::validator::ValidationConfig::default();

    let result = validator.validate_graph(&shapes_graph, &store, &config);
    assert!(result.is_ok());

    let report = result.unwrap();
    // Should conform since both shapes are satisfied
    assert!(report.conforms);
}
