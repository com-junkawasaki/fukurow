use fukurow_core::model::Triple;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> RdfStore {
        let mut store = RdfStore::new();
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

        let provenance = Provenance::Sensor {
            source: "test-sensor".to_string(),
            confidence: Some(0.9),
        };

        store.insert(triple1, GraphId::Default, provenance.clone());
        store.insert(triple2, GraphId::Named("test_graph".to_string()), provenance);

        store
    }

    #[tokio::test]
    async fn test_persistence_manager_memory_backend() {
        let backend = PersistenceBackend::Memory;
        let pm = PersistenceManager::new(backend);

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

        let backend = PersistenceBackend::Sqlite { url: db_path };
        let pm = PersistenceManager::new(backend);

        let store = create_test_store();
        assert_eq!(store.statistics().total_triples, 2);

        // Save store
        pm.save_store(&store).await.unwrap();

        // Load store
        let loaded = pm.load_store().await.unwrap();
        assert_eq!(loaded.statistics().total_triples, 2);
        assert_eq!(loaded.statistics().graph_count, 2);

        // Verify triples
        let default_triples = loaded.get_graph(&GraphId::Default);
        assert_eq!(default_triples.len(), 1);

        let named_triples = loaded.get_graph(&GraphId::Named("test_graph".to_string()));
        assert_eq!(named_triples.len(), 1);
    }

    #[cfg(feature = "turso")]
    #[tokio::test]
    async fn test_persistence_manager_turso_backend() {
        // This would require a real Turso URL, so we'll just test backend creation
        let backend = PersistenceBackend::Turso {
            url: "libsql://test.turso.io".to_string()
        };
        let pm = PersistenceManager::new(backend);

        // Save should fail with not implemented for now
        let store = create_test_store();
        let result = pm.save_store(&store).await;
        assert!(result.is_err());

        let result = pm.load_store().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_persistence_manager_sled_backend() {
        let backend = PersistenceBackend::Sled {
            path: "/tmp/test_sled".to_string()
        };
        let pm = PersistenceManager::new(backend);

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

        let adapter = crate::adapter::sqlite::SqliteAdapter::new(&db_path).await.unwrap();

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
                Provenance::Sensor { source, confidence } => {
                    assert_eq!(source, "test-sensor");
                    assert_eq!(confidence, &Some(0.9));
                }
                _ => panic!("Expected Sensor provenance"),
            }
        }
    }

    #[tokio::test]
    async fn test_sqlite_adapter_multiple_graphs() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = crate::adapter::sqlite::SqliteAdapter::new(&db_path).await.unwrap();

        let mut store = RdfStore::new();

        // Add triples to different graph types
        let triple1 = Triple {
            subject: "s1".to_string(),
            predicate: "p1".to_string(),
            object: "o1".to_string(),
        };
        let triple2 = Triple {
            subject: "s2".to_string(),
            predicate: "p2".to_string(),
            object: "o2".to_string(),
        };
        let triple3 = Triple {
            subject: "s3".to_string(),
            predicate: "p3".to_string(),
            object: "o3".to_string(),
        };
        let triple4 = Triple {
            subject: "s4".to_string(),
            predicate: "p4".to_string(),
            object: "o4".to_string(),
        };

        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: None,
        };

        store.insert(triple1, GraphId::Default, provenance.clone());
        store.insert(triple2, GraphId::Named("named".to_string()), provenance.clone());
        store.insert(triple3, GraphId::Sensor("sensor1".to_string()), provenance.clone());
        store.insert(triple4, GraphId::Inferred("rule1".to_string()), provenance.clone());

        // Save and load
        adapter.save_store(&store).await.unwrap();
        let loaded = adapter.load_store().await.unwrap();

        assert_eq!(loaded.statistics().total_triples, 4);
        assert_eq!(loaded.statistics().graph_count, 4);

        let graph_ids = loaded.graph_ids();
        assert_eq!(graph_ids.len(), 4);
        assert!(graph_ids.contains(&GraphId::Default));
        assert!(graph_ids.contains(&GraphId::Named("named".to_string())));
        assert!(graph_ids.contains(&GraphId::Sensor("sensor1".to_string())));
        assert!(graph_ids.contains(&GraphId::Inferred("rule1".to_string())));
    }

    #[tokio::test]
    async fn test_sqlite_adapter_empty_store() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = crate::adapter::sqlite::SqliteAdapter::new(&db_path).await.unwrap();
        let store = RdfStore::new();

        adapter.save_store(&store).await.unwrap();
        let loaded = adapter.load_store().await.unwrap();

        assert_eq!(loaded.statistics().total_triples, 0);
        assert_eq!(loaded.statistics().graph_count, 0);
    }
}
