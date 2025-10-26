#[cfg(test)]

use fukurow_core::model::Triple;
use chrono::{DateTime, Utc};

#[cfg(test)]
mod store_tests {
    use super::*;

    fn create_test_triple(subject: &str, predicate: &str, object: &str) -> Triple {
        Triple {
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
        }
    }

    fn create_test_provenance() -> fukurow_store::Provenance {
        fukurow_store::Provenance::Sensor {
            source: "test-sensor".to_string(),
            confidence: Some(0.9),
        }
    }

    #[test]
    fn test_rdf_store_creation() {
        let store = fukurow_store::RdfStore::new();
        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);
    }

    #[test]
    fn test_rdf_store_with_audit_limit() {
        let store = fukurow_store::RdfStore::with_audit_limit(50);
        assert_eq!(store.audit_trail().len(), 0);
    }

    #[test]
    fn test_insert_single_triple() {
        let mut store = fukurow_store::RdfStore::new();
        let triple = create_test_triple("s1", "p1", "o1");
        let provenance = create_test_provenance();

        store.insert(triple.clone(), fukurow_store::GraphId::Default, provenance.clone());

        assert_eq!(store.statistics().total_triples, 1);
        assert_eq!(store.statistics().graph_count, 1);

        let results = store.find_triples(Some("s1"), Some("p1"), Some("o1"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].triple, triple);
        assert_eq!(results[0].provenance, provenance);
    }

    #[test]
    fn test_insert_multiple_graphs() {
        let mut store = fukurow_store::RdfStore::new();

        // Insert to default graph
        let triple1 = create_test_triple("s1", "p1", "o1");
        store.insert(triple1, fukurow_store::GraphId::Default, create_test_provenance());

        // Insert to named graph
        let triple2 = create_test_triple("s2", "p2", "o2");
        store.insert(triple2, fukurow_store::GraphId::Named("graph1".to_string()), create_test_provenance());

        // Insert to sensor graph
        let triple3 = create_test_triple("s3", "p3", "o3");
        store.insert(triple3, fukurow_store::GraphId::Sensor("sensor1".to_string()), create_test_provenance());

        // Insert to inferred graph
        let triple4 = create_test_triple("s4", "p4", "o4");
        store.insert(triple4, fukurow_store::GraphId::Inferred("rule1".to_string()), create_test_provenance());

        assert_eq!(store.statistics().total_triples, 4);
        assert_eq!(store.statistics().graph_count, 4);

        let graph_ids = store.graph_ids();
        assert_eq!(graph_ids.len(), 4);
    }

    #[test]
    fn test_find_triples_subject_only() {
        let mut store = fukurow_store::RdfStore::new();
        let triple1 = create_test_triple("s1", "p1", "o1");
        let triple2 = create_test_triple("s1", "p2", "o2");
        let triple3 = create_test_triple("s2", "p1", "o1");

        store.insert(triple1.clone(), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(triple2.clone(), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(triple3, fukurow_store::GraphId::Default, create_test_provenance());

        let results = store.find_triples(Some("s1"), None, None);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.triple.predicate == "p1"));
        assert!(results.iter().any(|r| r.triple.predicate == "p2"));
    }

    #[test]
    fn test_find_triples_predicate_only() {
        let mut store = fukurow_store::RdfStore::new();
        let triple1 = create_test_triple("s1", "p1", "o1");
        let triple2 = create_test_triple("s2", "p1", "o2");
        let triple3 = create_test_triple("s1", "p2", "o1");

        store.insert(triple1.clone(), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(triple2.clone(), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(triple3, fukurow_store::GraphId::Default, create_test_provenance());

        let results = store.find_triples(None, Some("p1"), None);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.triple.subject == "s1"));
        assert!(results.iter().any(|r| r.triple.subject == "s2"));
    }

    #[test]
    fn test_find_triples_object_only() {
        let mut store = fukurow_store::RdfStore::new();
        let triple1 = create_test_triple("s1", "p1", "o1");
        let triple2 = create_test_triple("s2", "p2", "o1");
        let triple3 = create_test_triple("s1", "p2", "o2");

        store.insert(triple1.clone(), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(triple2.clone(), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(triple3, fukurow_store::GraphId::Default, create_test_provenance());

        let results = store.find_triples(None, None, Some("o1"));
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.triple.subject == "s1"));
        assert!(results.iter().any(|r| r.triple.subject == "s2"));
    }

    #[test]
    fn test_find_triples_no_filter() {
        let mut store = fukurow_store::RdfStore::new();
        store.insert(create_test_triple("s1", "p1", "o1"), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), fukurow_store::GraphId::Default, create_test_provenance());

        let results = store.find_triples(None, None, None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_get_graph() {
        let mut store = fukurow_store::RdfStore::new();
        let graph_id = fukurow_store::GraphId::Named("test_graph".to_string());

        store.insert(create_test_triple("s1", "p1", "o1"), graph_id.clone(), create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), graph_id.clone(), create_test_provenance());

        let graph = store.get_graph(&graph_id);
        assert_eq!(graph.len(), 2);

        let non_existent = store.get_graph(&fukurow_store::GraphId::Named("non_existent".to_string()));
        assert_eq!(non_existent.len(), 0);
    }

    #[test]
    fn test_get_default_graph() {
        let mut store = fukurow_store::RdfStore::new();
        store.insert(create_test_triple("s1", "p1", "o1"), fukurow_store::GraphId::Default, create_test_provenance());

        let default_graph = store.get_graph(&fukurow_store::GraphId::Default);
        assert_eq!(default_graph.len(), 1);
    }

    #[test]
    fn test_clear_graph() {
        let mut store = fukurow_store::RdfStore::new();
        let graph_id = fukurow_store::GraphId::Named("test_graph".to_string());

        store.insert(create_test_triple("s1", "p1", "o1"), graph_id.clone(), create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), graph_id.clone(), create_test_provenance());

        assert_eq!(store.statistics().total_triples, 2);
        assert_eq!(store.statistics().graph_count, 1);
        assert_eq!(store.audit_trail().len(), 2); // 2 inserts

        store.clear_graph(&graph_id);

        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);
        assert_eq!(store.audit_trail().len(), 3); // 2 inserts + 1 clear
    }

    #[test]
    fn test_clear_all() {
        let mut store = fukurow_store::RdfStore::new();

        store.insert(create_test_triple("s1", "p1", "o1"), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), fukurow_store::GraphId::Named("g1".to_string()), create_test_provenance());

        assert_eq!(store.statistics().total_triples, 2);
        assert_eq!(store.statistics().graph_count, 2);
        assert_eq!(store.audit_trail().len(), 2); // 2 inserts

        store.clear_all();

        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);
        assert_eq!(store.audit_trail().len(), 3); // 2 inserts + 1 clear
    }

    #[test]
    fn test_audit_trail_limit() {
        let mut store = fukurow_store::RdfStore::with_audit_limit(2);

        store.insert(create_test_triple("s1", "p1", "o1"), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s3", "p3", "o3"), fukurow_store::GraphId::Default, create_test_provenance());

        assert_eq!(store.audit_trail().len(), 2); // Should be limited to 2
    }

    #[test]
    fn test_set_audit_limit() {
        let mut store = fukurow_store::RdfStore::new();

        store.insert(create_test_triple("s1", "p1", "o1"), fukurow_store::GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), fukurow_store::GraphId::Default, create_test_provenance());

        assert_eq!(store.audit_trail().len(), 2);

        store.set_audit_limit(1);
        assert_eq!(store.audit_trail().len(), 1);
    }

    #[test]
    fn test_all_triples() {
        let mut store = fukurow_store::RdfStore::new();
        let graph_id1 = fukurow_store::GraphId::Default;
        let graph_id2 = fukurow_store::GraphId::Named("g1".to_string());

        store.insert(create_test_triple("s1", "p1", "o1"), graph_id1.clone(), create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), graph_id2.clone(), create_test_provenance());

        let all_triples = store.all_triples();
        assert_eq!(all_triples.len(), 2);
        assert!(all_triples.contains_key(&graph_id1));
        assert!(all_triples.contains_key(&graph_id2));
    }
}

#[cfg(test)]
mod provenance_tests {
    use super::*;

    #[test]
    fn test_provenance_sensor() {
        let provenance = fukurow_store::Provenance::Sensor {
            source: "test-sensor".to_string(),
            confidence: Some(0.85),
        };

        match provenance {
            fukurow_store::Provenance::Sensor { source, confidence } => {
                assert_eq!(source, "test-sensor");
                assert_eq!(confidence, Some(0.85));
            }
            _ => panic!("Expected Sensor provenance"),
        }
    }

    #[test]
    fn test_provenance_sensor_no_confidence() {
        let provenance = fukurow_store::Provenance::Sensor {
            source: "test-sensor".to_string(),
            confidence: None,
        };

        match provenance {
            fukurow_store::Provenance::Sensor { source, confidence } => {
                assert_eq!(source, "test-sensor");
                assert_eq!(confidence, None);
            }
            _ => panic!("Expected Sensor provenance"),
        }
    }

    #[test]
    fn test_provenance_inferred() {
        let evidence = vec!["triple1".to_string(), "triple2".to_string()];
        let provenance = fukurow_store::Provenance::Inferred {
            rule: "test-rule".to_string(),
            reasoning_level: "rdfs".to_string(),
            evidence: evidence.clone(),
        };

        match provenance {
            fukurow_store::Provenance::Inferred { rule, reasoning_level, evidence: ev } => {
                assert_eq!(rule, "test-rule");
                assert_eq!(reasoning_level, "rdfs");
                assert_eq!(ev, evidence);
            }
            _ => panic!("Expected Inferred provenance"),
        }
    }

    #[test]
    fn test_provenance_imported() {
        let imported_at = Utc::now();
        let provenance = fukurow_store::Provenance::Imported {
            source_uri: "file://test.ttl".to_string(),
            imported_at,
        };

        match provenance {
            fukurow_store::Provenance::Imported { source_uri, imported_at: at } => {
                assert_eq!(source_uri, "file://test.ttl");
                assert_eq!(at, imported_at);
            }
            _ => panic!("Expected Imported provenance"),
        }
    }

    #[test]
    fn test_graph_id_default() {
        let graph_id = fukurow_store::GraphId::Default;
        assert_eq!(format!("{}", graph_id), "default");
    }

    #[test]
    fn test_graph_id_named() {
        let graph_id = fukurow_store::GraphId::Named("test_graph".to_string());
        assert_eq!(format!("{}", graph_id), "named:test_graph");
    }

    #[test]
    fn test_graph_id_sensor() {
        let graph_id = fukurow_store::GraphId::Sensor("sensor_001".to_string());
        assert_eq!(format!("{}", graph_id), "sensor:sensor_001");
    }

    #[test]
    fn test_graph_id_inferred() {
        let graph_id = fukurow_store::GraphId::Inferred("rule_001".to_string());
        assert_eq!(format!("{}", graph_id), "inferred:rule_001");
    }

    #[test]
    fn test_graph_id_default_equality() {
        assert_eq!(fukurow_store::GraphId::Default, fukurow_store::GraphId::Default);
        assert_ne!(fukurow_store::GraphId::Default, fukurow_store::GraphId::Named("test".to_string()));
    }

    #[test]
    fn test_graph_id_named_equality() {
        let g1 = fukurow_store::GraphId::Named("test".to_string());
        let g2 = fukurow_store::GraphId::Named("test".to_string());
        let g3 = fukurow_store::GraphId::Named("other".to_string());

        assert_eq!(g1, g2);
        assert_ne!(g1, g3);
    }

    #[test]
    fn test_audit_entry_insert() {
        let timestamp = Utc::now();
        let triple_str = "s p o";
        let graph_id = fukurow_store::GraphId::Default;
        let provenance = fukurow_store::Provenance::Sensor {
            source: "test".to_string(),
            confidence: None,
        };

        let operation = fukurow_store::AuditOperation::Insert {
            triple: triple_str.to_string(),
            graph_id: graph_id.clone(),
            provenance: provenance.clone(),
        };

        let entry = fukurow_store::AuditEntry {
            id: "test-id".to_string(),
            timestamp,
            operation,
            actor: Some("test-user".to_string()),
            metadata: std::collections::HashMap::new(),
        };

        assert_eq!(entry.id, "test-id");
        assert_eq!(entry.timestamp, timestamp);
        assert_eq!(entry.actor, Some("test-user".to_string()));

        match entry.operation {
            fukurow_store::AuditOperation::Insert { triple, graph_id: gid, provenance: prov } => {
                assert_eq!(triple, triple_str);
                assert_eq!(gid, graph_id);
                assert_eq!(prov, provenance);
            }
            _ => panic!("Expected Insert operation"),
        }
    }

    #[test]
    fn test_audit_entry_delete() {
        let timestamp = Utc::now();
        let triple_str = "s p o";
        let graph_id = fukurow_store::GraphId::Named("test".to_string());

        let operation = fukurow_store::AuditOperation::Delete {
            triple: triple_str.to_string(),
            graph_id: graph_id.clone(),
        };

        let entry = fukurow_store::AuditEntry {
            id: "test-id".to_string(),
            timestamp,
            operation,
            actor: None,
            metadata: std::collections::HashMap::new(),
        };

        match entry.operation {
            fukurow_store::AuditOperation::Delete { triple, graph_id: gid } => {
                assert_eq!(triple, triple_str);
                assert_eq!(gid, graph_id);
            }
            _ => panic!("Expected Delete operation"),
        }
    }

    #[test]
    fn test_audit_entry_clear() {
        let timestamp = Utc::now();
        let graph_id = fukurow_store::GraphId::Sensor("sensor1".to_string());

        let operation = fukurow_store::AuditOperation::Clear {
            graph_id: graph_id.clone(),
            triple_count: 42,
        };

        let entry = fukurow_store::AuditEntry {
            id: "test-id".to_string(),
            timestamp,
            operation,
            actor: None,
            metadata: std::collections::HashMap::new(),
        };

        match entry.operation {
            fukurow_store::AuditOperation::Clear { graph_id: gid, triple_count } => {
                assert_eq!(gid, graph_id);
                assert_eq!(triple_count, 42);
            }
            _ => panic!("Expected Clear operation"),
        }
    }

    #[test]
    fn test_audit_entry_inference() {
        let timestamp = Utc::now();

        let operation = fukurow_store::AuditOperation::Inference {
            rule: "test-rule".to_string(),
            triples_added: 10,
            triples_removed: 2,
        };

        let entry = fukurow_store::AuditEntry {
            id: "test-id".to_string(),
            timestamp,
            operation,
            actor: None,
            metadata: std::collections::HashMap::new(),
        };

        match entry.operation {
            fukurow_store::AuditOperation::Inference { rule, triples_added, triples_removed } => {
                assert_eq!(rule, "test-rule");
                assert_eq!(triples_added, 10);
                assert_eq!(triples_removed, 2);
            }
            _ => panic!("Expected Inference operation"),
        }
    }

    #[test]
    fn test_audit_entry_query() {
        let timestamp = Utc::now();

        let operation = fukurow_store::AuditOperation::Query {
            query_type: "SPARQL".to_string(),
            result_count: 100,
        };

        let entry = fukurow_store::AuditEntry {
            id: "test-id".to_string(),
            timestamp,
            operation,
            actor: None,
            metadata: std::collections::HashMap::new(),
        };

        match entry.operation {
            fukurow_store::AuditOperation::Query { query_type, result_count } => {
                assert_eq!(query_type, "SPARQL");
                assert_eq!(result_count, 100);
            }
            _ => panic!("Expected Query operation"),
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use fukurow_store::adapter::StoreAdapter;

    fn create_test_store() -> fukurow_store::RdfStore {
        let mut store = fukurow_store::RdfStore::new();
        let triple1 = Triple {
            subject: "http://example.org/s1".to_string(),
            predicate: "http://example.org/p1".to_string(),
            object: "http://example.org/o1".to_string(),
        };
        let triple2 = Triple {
            subject: "http://example.org/s2".to_string(),
            predicate: "http://example.org/p2".to_string(),
            object: "http://example.org/o2".to_string(),
        };

        let provenance = fukurow_store::Provenance::Sensor {
            source: "test-sensor".to_string(),
            confidence: Some(0.9),
        };

        store.insert(triple1, fukurow_store::GraphId::Default, provenance.clone());
        store.insert(triple2, fukurow_store::GraphId::Named("test_graph".to_string()), provenance);

        store
    }

    #[tokio::test]
    async fn test_persistence_manager_memory_backend() {
        let backend = fukurow_store::PersistenceBackend::Memory;
        let pm = fukurow_store::PersistenceManager::new(backend);

        let store = create_test_store();
        assert_eq!(store.statistics().total_triples, 2);

        // Save should be no-op for memory
        pm.save_store(&store).await.unwrap();

        // Load should return empty store for memory
        let loaded = pm.load_store().await.unwrap();
        assert_eq!(loaded.statistics().total_triples, 0);
    }

    #[tokio::test]
    async fn test_persistence_manager_sqlite_backend() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let backend = fukurow_store::PersistenceBackend::Sqlite { url: db_path };
        let pm = fukurow_store::PersistenceManager::new(backend);

        let store = create_test_store();
        assert_eq!(store.statistics().total_triples, 2);

        // Save store
        pm.save_store(&store).await.unwrap();

        // Load store
        let loaded = pm.load_store().await.unwrap();
        assert_eq!(loaded.statistics().total_triples, 2);
        assert_eq!(loaded.statistics().graph_count, 2);

        // Verify triples
        let default_triples = loaded.get_graph(&fukurow_store::GraphId::Default);
        assert_eq!(default_triples.len(), 1);

        let named_triples = loaded.get_graph(&fukurow_store::GraphId::Named("test_graph".to_string()));
        assert_eq!(named_triples.len(), 1);
    }

    #[cfg(feature = "turso")]
    #[tokio::test]
    async fn test_persistence_manager_turso_backend() {
        // This would require a real Turso URL, so we'll just test backend creation
        let backend = fukurow_store::PersistenceBackend::Turso {
            url: "libsql://test.turso.io".to_string()
        };
        let pm = fukurow_store::PersistenceManager::new(backend);

        // Save should fail with not implemented for now
        let store = create_test_store();
        let result = pm.save_store(&store).await;
        assert!(result.is_err());

        let result = pm.load_store().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_persistence_manager_sled_backend() {
        let backend = fukurow_store::PersistenceBackend::Sled {
            path: "/tmp/test_sled".to_string()
        };
        let pm = fukurow_store::PersistenceManager::new(backend);

        let store = create_test_store();

        // Save should fail with not implemented
        let result = pm.save_store(&store).await;
        assert!(result.is_err());

        // Load should fail with not implemented
        let result = pm.load_store().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sqlite_adapter_basic_operations() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = fukurow_store::SqliteAdapter::new(&db_path).await.unwrap();

        let store = create_test_store();

        // Save
        adapter.save_store(&store).await.unwrap();

        // Load
        let loaded = adapter.load_store().await.unwrap();
        assert_eq!(loaded.statistics().total_triples, 2);

        // Verify provenance
        let triples = loaded.find_triples(None, None, None);
        assert_eq!(triples.len(), 2);
        for triple in triples {
            match &triple.provenance {
                fukurow_store::Provenance::Sensor { source, confidence } => {
                    assert_eq!(source, "test-sensor");
                    assert_eq!(*confidence, Some(0.9));
                }
                _ => panic!("Expected Sensor provenance"),
            }
        }
    }

    #[tokio::test]
    async fn test_sqlite_adapter_multiple_graphs() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = fukurow_store::SqliteAdapter::new(&db_path).await.unwrap();

        let mut store = fukurow_store::RdfStore::new();

        // Add triples to different graph types
        let triples = vec![
            (fukurow_store::GraphId::Default, "s1"),
            (fukurow_store::GraphId::Named("named".to_string()), "s2"),
            (fukurow_store::GraphId::Sensor("sensor1".to_string()), "s3"),
            (fukurow_store::GraphId::Inferred("rule1".to_string()), "s4"),
        ];

        for (graph_id, subject) in triples {
            let triple = Triple {
                subject: subject.to_string(),
                predicate: "p".to_string(),
                object: "o".to_string(),
            };
            let provenance = fukurow_store::Provenance::Sensor {
                source: "test".to_string(),
                confidence: None,
            };
            store.insert(triple, graph_id, provenance);
        }

        // Save and load
        adapter.save_store(&store).await.unwrap();
        let loaded = adapter.load_store().await.unwrap();

        assert_eq!(loaded.statistics().total_triples, 4);
        assert_eq!(loaded.statistics().graph_count, 4);
    }

    #[tokio::test]
    async fn test_sqlite_adapter_empty_store() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = fukurow_store::SqliteAdapter::new(&db_path).await.unwrap();
        let store = fukurow_store::RdfStore::new();

        adapter.save_store(&store).await.unwrap();
        let loaded = adapter.load_store().await.unwrap();

        assert_eq!(loaded.statistics().total_triples, 0);
        assert_eq!(loaded.statistics().graph_count, 0);
    }

    #[tokio::test]
    async fn test_sqlite_adapter_creation() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = fukurow_store::SqliteAdapter::new(&db_path).await;
        assert!(adapter.is_ok());
    }

    #[tokio::test]
    async fn test_sqlite_adapter_save_load_empty_store() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = fukurow_store::SqliteAdapter::new(&db_path).await.unwrap();
        let store = fukurow_store::RdfStore::new();

        // Save empty store
        adapter.save_store(&store).await.unwrap();

        // Load empty store
        let loaded = adapter.load_store().await.unwrap();
        assert_eq!(loaded.statistics().total_triples, 0);
        assert_eq!(loaded.statistics().graph_count, 0);
    }

    #[tokio::test]
    async fn test_sqlite_adapter_save_load_with_data() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = fukurow_store::SqliteAdapter::new(&db_path).await.unwrap();

        // Create store with data
        let mut store = fukurow_store::RdfStore::new();
        let triple1 = Triple {
            subject: "http://example.org/subject1".to_string(),
            predicate: "http://example.org/predicate1".to_string(),
            object: "http://example.org/object1".to_string(),
        };
        let triple2 = Triple {
            subject: "http://example.org/subject2".to_string(),
            predicate: "http://example.org/predicate2".to_string(),
            object: "http://example.org/object2".to_string(),
        };

        store.insert(triple1.clone(), fukurow_store::GraphId::Default, fukurow_store::Provenance::Sensor {
            source: "test_sensor".to_string(),
            confidence: Some(0.8),
        });
        store.insert(triple2.clone(), fukurow_store::GraphId::Named("named_graph".to_string()), fukurow_store::Provenance::Inferred {
            rule: "test_rule".to_string(),
            reasoning_level: "rdfs".to_string(),
            evidence: vec!["evidence1".to_string()],
        });

        // Save store
        adapter.save_store(&store).await.unwrap();

        // Load store
        let loaded = adapter.load_store().await.unwrap();

        // Verify data
        assert_eq!(loaded.statistics().total_triples, 2);
        assert_eq!(loaded.statistics().graph_count, 2);

        // Check default graph
        let default_triples = loaded.find_triples(Some("http://example.org/subject1"), None, None);
        assert_eq!(default_triples.len(), 1);
        assert_eq!(default_triples[0].triple, triple1);
        match &default_triples[0].provenance {
            fukurow_store::Provenance::Sensor { source, confidence } => {
                assert_eq!(source, "test_sensor");
                assert_eq!(*confidence, Some(0.8));
            }
            _ => panic!("Expected Sensor provenance"),
        }

        // Check named graph
        let named_triples = loaded.find_triples(Some("http://example.org/subject2"), None, None);
        assert_eq!(named_triples.len(), 1);
        assert_eq!(named_triples[0].triple, triple2);
        match &named_triples[0].provenance {
            fukurow_store::Provenance::Inferred { rule, reasoning_level, evidence } => {
                assert_eq!(rule, "test_rule");
                assert_eq!(reasoning_level, "rdfs");
                assert_eq!(evidence, &vec!["evidence1".to_string()]);
            }
            _ => panic!("Expected Inferred provenance"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_adapter_overwrite_data() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = fukurow_store::SqliteAdapter::new(&db_path).await.unwrap();

        // Save initial data
        let mut store1 = fukurow_store::RdfStore::new();
        store1.insert(Triple {
            subject: "s1".to_string(),
            predicate: "p1".to_string(),
            object: "o1".to_string(),
        }, fukurow_store::GraphId::Default, fukurow_store::Provenance::Sensor {
            source: "test".to_string(),
            confidence: None,
        });
        adapter.save_store(&store1).await.unwrap();

        // Save different data (should overwrite)
        let mut store2 = fukurow_store::RdfStore::new();
        store2.insert(Triple {
            subject: "s2".to_string(),
            predicate: "p2".to_string(),
            object: "o2".to_string(),
        }, fukurow_store::GraphId::Default, fukurow_store::Provenance::Sensor {
            source: "test".to_string(),
            confidence: None,
        });
        adapter.save_store(&store2).await.unwrap();

        // Load and verify only new data exists
        let loaded = adapter.load_store().await.unwrap();
        assert_eq!(loaded.statistics().total_triples, 1);

        let results = loaded.find_triples(Some("s2"), Some("p2"), Some("o2"));
        assert_eq!(results.len(), 1);

        let results_old = loaded.find_triples(Some("s1"), None, None);
        assert_eq!(results_old.len(), 0);
    }

    #[tokio::test]
    async fn test_sqlite_adapter_multiple_graph_types() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = fukurow_store::SqliteAdapter::new(&db_path).await.unwrap();

        let mut store = fukurow_store::RdfStore::new();

        // Add triples to all graph types
        let graphs = vec![
            fukurow_store::GraphId::Default,
            fukurow_store::GraphId::Named("named".to_string()),
            fukurow_store::GraphId::Sensor("sensor".to_string()),
            fukurow_store::GraphId::Inferred("rule".to_string()),
        ];

        for (i, graph_id) in graphs.into_iter().enumerate() {
            let triple = Triple {
                subject: format!("subject{}", i),
                predicate: format!("predicate{}", i),
                object: format!("object{}", i),
            };
            store.insert(triple, graph_id, fukurow_store::Provenance::Sensor {
                source: "test".to_string(),
                confidence: None,
            });
        }

        // Save and load
        adapter.save_store(&store).await.unwrap();
        let loaded = adapter.load_store().await.unwrap();

        assert_eq!(loaded.statistics().total_triples, 4);
        assert_eq!(loaded.statistics().graph_count, 4);

        // Verify all graph IDs exist
        let graph_ids = loaded.graph_ids();
        assert_eq!(graph_ids.len(), 4);
        assert!(graph_ids.contains(&&fukurow_store::GraphId::Default));
        assert!(graph_ids.contains(&&fukurow_store::GraphId::Named("named".to_string())));
        assert!(graph_ids.contains(&&fukurow_store::GraphId::Sensor("sensor".to_string())));
        assert!(graph_ids.contains(&&fukurow_store::GraphId::Inferred("rule".to_string())));
    }

    #[tokio::test]
    async fn test_sqlite_adapter_schema_creation() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        // First adapter creation should create schema
        let adapter1 = fukurow_store::SqliteAdapter::new(&db_path).await.unwrap();

        // Second adapter creation should reuse existing schema
        let adapter2 = fukurow_store::SqliteAdapter::new(&db_path).await.unwrap();

        // Both should work
        let store = fukurow_store::RdfStore::new();
        adapter1.save_store(&store).await.unwrap();
        adapter2.save_store(&store).await.unwrap();

        let loaded = adapter2.load_store().await.unwrap();
        assert_eq!(loaded.statistics().total_triples, 0);
    }
}
