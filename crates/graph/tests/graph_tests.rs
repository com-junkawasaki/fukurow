//! Tests for the graph crate

use reasoner_graph::model::{Triple, NamedGraph, JsonLdDocument};
use reasoner_graph::store::GraphStore;
use reasoner_graph::query::{GraphQuery, var, const_val};

#[test]
fn test_triple_creation() {
    let triple = Triple {
        subject: "https://example.com/user/1".to_string(),
        predicate: "https://example.com/ns/name".to_string(),
        object: "Alice".to_string(),
    };

    assert_eq!(triple.subject, "https://example.com/user/1");
    assert_eq!(triple.predicate, "https://example.com/ns/name");
    assert_eq!(triple.object, "Alice");
}

#[test]
fn test_named_graph_creation() {
    let triples = vec![
        Triple {
            subject: "https://example.com/user/1".to_string(),
            predicate: "https://example.com/ns/name".to_string(),
            object: "Alice".to_string(),
        },
        Triple {
            subject: "https://example.com/user/1".to_string(),
            predicate: "https://example.com/ns/age".to_string(),
            object: "30".to_string(),
        },
    ];

    let named_graph = NamedGraph {
        name: "users".to_string(),
        triples,
    };

    assert_eq!(named_graph.name, "users");
    assert_eq!(named_graph.triples.len(), 2);
}

#[test]
fn test_graph_store_operations() {
    let mut store = GraphStore::new();

    // Add triples to default graph
    let triple1 = Triple {
        subject: "https://example.com/user/1".to_string(),
        predicate: "https://example.com/ns/name".to_string(),
        object: "Alice".to_string(),
    };

    let triple2 = Triple {
        subject: "https://example.com/user/1".to_string(),
        predicate: "https://example.com/ns/age".to_string(),
        object: "30".to_string(),
    };

    store.add_triple(triple1);
    store.add_triple(triple2);

    // Check default graph
    let default_graph = store.get_default_graph();
    assert_eq!(default_graph.triples.len(), 2);

    // Add to named graph
    let triple3 = Triple {
        subject: "https://example.com/user/2".to_string(),
        predicate: "https://example.com/ns/name".to_string(),
        object: "Bob".to_string(),
    };

    store.add_triple_to_graph("users", triple3);

    let users_graph = store.get_graph("users").unwrap();
    assert_eq!(users_graph.triples.len(), 1);
    assert_eq!(users_graph.triples[0].object, "Bob");
}

#[test]
fn test_graph_queries() {
    let mut store = GraphStore::new();

    // Add test data
    store.add_triple(Triple {
        subject: "https://example.com/user/1".to_string(),
        predicate: "https://example.com/ns/name".to_string(),
        object: "Alice".to_string(),
    });

    store.add_triple(Triple {
        subject: "https://example.com/user/1".to_string(),
        predicate: "https://example.com/ns/age".to_string(),
        object: "30".to_string(),
    });

    store.add_triple(Triple {
        subject: "https://example.com/user/2".to_string(),
        predicate: "https://example.com/ns/name".to_string(),
        object: "Bob".to_string(),
    });

    // Query all triples
    let all_triples = store.find_triples(None, None, None);
    assert_eq!(all_triples.len(), 3);

    // Query by subject
    let alice_triples = store.find_triples(Some("https://example.com/user/1"), None, None);
    assert_eq!(alice_triples.len(), 2);

    // Query by predicate
    let name_triples = store.find_triples(None, Some("https://example.com/ns/name"), None);
    assert_eq!(name_triples.len(), 2);

    // Query by object
    let alice_name = store.find_triples(None, None, Some("Alice"));
    assert_eq!(alice_name.len(), 1);
    assert_eq!(alice_name[0].subject, "https://example.com/user/1");
}

#[test]
fn test_graph_query_builder() {
    let mut store = GraphStore::new();

    // Add test data
    store.add_triple(Triple {
        subject: "https://example.com/user/1".to_string(),
        predicate: "https://example.com/ns/knows".to_string(),
        object: "https://example.com/user/2".to_string(),
    });

    store.add_triple(Triple {
        subject: "https://example.com/user/2".to_string(),
        predicate: "https://example.com/ns/knows".to_string(),
        object: "https://example.com/user/3".to_string(),
    });

    // Build query: Find friends of friends
    let query = GraphQuery::new()
        .where_clause(
            var("person1"),
            const_val("https://example.com/ns/knows"),
            var("person2"),
        )
        .where_clause(
            var("person2"),
            const_val("https://example.com/ns/knows"),
            var("person3"),
        );

    let results = query.execute(&store);
    assert!(!results.is_empty());

    // Check that we found the friend-of-friend relationship
    for result in results {
        if let (Some(person1), Some(person2), Some(person3)) = (
            result.get("person1"),
            result.get("person2"),
            result.get("person3"),
        ) {
            assert_eq!(person1, "https://example.com/user/1");
            assert_eq!(person2, "https://example.com/user/2");
            assert_eq!(person3, "https://example.com/user/3");
        }
    }
}

#[test]
fn test_jsonld_conversion() {
    let mut store = GraphStore::new();

    store.add_triple(Triple {
        subject: "https://example.com/user/1".to_string(),
        predicate: "https://example.com/ns/name".to_string(),
        object: "Alice".to_string(),
    });

    // Convert to JSON-LD
    let jsonld_result = store.to_jsonld();
    assert!(jsonld_result.is_ok());

    let jsonld_doc = jsonld_result.unwrap();
    assert_eq!(jsonld_doc.context["@vocab"], "https://w3id.org/security#");

    // The graph should contain our triple
    if let Some(graph) = &jsonld_doc.graph {
        assert!(!graph.is_empty());
    }
}

#[test]
fn test_graph_store_clear() {
    let mut store = GraphStore::new();

    store.add_triple(Triple {
        subject: "https://example.com/test".to_string(),
        predicate: "https://example.com/ns/test".to_string(),
        object: "value".to_string(),
    });

    store.add_triple_to_graph("test_graph", Triple {
        subject: "https://example.com/test2".to_string(),
        predicate: "https://example.com/ns/test2".to_string(),
        object: "value2".to_string(),
    });

    // Verify data exists
    assert_eq!(store.get_default_graph().triples.len(), 1);
    assert!(store.get_graph("test_graph").is_some());

    // Clear and verify
    store.clear();
    assert_eq!(store.get_default_graph().triples.len(), 0);
    assert!(store.get_graph("test_graph").is_none());
    assert!(store.graph_names().is_empty());
}

#[test]
fn test_cyber_event_to_jsonld() {
    use reasoner_graph::model::CyberEvent;

    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.10".to_string(),
        dest_ip: "10.0.0.50".to_string(),
        port: 443,
        protocol: "tcp".to_string(),
        timestamp: 1640995200,
    };

    let jsonld_result = reasoner_graph::jsonld::cyber_event_to_jsonld(&event);
    assert!(jsonld_result.is_ok());

    let doc = jsonld_result.unwrap();
    assert!(doc.context.get("@vocab").is_some());
}

#[test]
fn test_jsonld_parsing() {
    let json_str = r#"{
        "@context": {
            "@vocab": "https://w3id.org/security#"
        },
        "@graph": [
            {
                "@id": "https://example.com/test",
                "name": "Test Resource"
            }
        ]
    }"#;

    let parse_result = reasoner_graph::jsonld::parse_jsonld(json_str);
    assert!(parse_result.is_ok());

    let doc = parse_result.unwrap();
    assert_eq!(doc.context["@vocab"], "https://w3id.org/security#");
}
