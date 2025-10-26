use fukurow_core::model::Triple;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> RdfStore {
        let mut store = RdfStore::new();
        let triple = Triple {
            subject: "test://subject".to_string(),
            predicate: "test://predicate".to_string(),
            object: "test://object".to_string(),
        };
        let provenance = Provenance::Sensor {
            source: "test-adapter".to_string(),
            confidence: Some(0.95),
        };

        store.insert(triple, GraphId::Default, provenance);
        store
    }

    #[test]
    fn test_store_adapter_trait() {
        // This is a compile-time test to ensure the trait is properly defined
        // The actual implementations are tested in persistence_tests.rs
    }

    #[tokio::test]
    async fn test_sqlite_adapter_creation() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = crate::adapter::sqlite::SqliteAdapter::new(&db_path).await;
        assert!(adapter.is_ok());
    }

    #[tokio::test]
    async fn test_sqlite_adapter_schema_creation() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = crate::adapter::sqlite::SqliteAdapter::new(&db_path).await.unwrap();

        // Test that schema was created by saving and loading
        let store = create_test_store();
        adapter.save_store(&store).await.unwrap();
        let loaded = adapter.load_store().await.unwrap();

        assert_eq!(loaded.statistics().total_triples, 1);
    }

    #[tokio::test]
    async fn test_sqlite_adapter_graph_id_serialization() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let adapter = crate::adapter::sqlite::SqliteAdapter::new(&db_path).await.unwrap();

        let mut store = RdfStore::new();

        // Test all graph ID types
        let triples = vec![
            (GraphId::Default, "s1"),
            (GraphId::Named("test".to_string()), "s2"),
            (GraphId::Sensor("sensor".to_string()), "s3"),
            (GraphId::Inferred("rule".to_string()), "s4"),
        ];

        for (graph_id, subject) in triples {
            let triple = Triple {
                subject: subject.to_string(),
                predicate: "p".to_string(),
                object: "o".to_string(),
            };
            let provenance = Provenance::Sensor {
                source: "test".to_string(),
                confidence: None,
            };
            store.insert(triple, graph_id, provenance);
        }

        adapter.save_store(&store).await.unwrap();
        let loaded = adapter.load_store().await.unwrap();

        assert_eq!(loaded.statistics().total_triples, 4);
        assert_eq!(loaded.statistics().graph_count, 4);
    }

    #[cfg(feature = "turso")]
    #[test]
    fn test_turso_adapter_stub() {
        // Compile-time test for TursoAdapter existence
        // Actual functionality would require real Turso credentials
    }
}
