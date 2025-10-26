//! # Fukurow Store
//!
//! Provenance付きRDF Triple Store
//! 観測事実・推論事実を格納し、監査・トレーサビリティを確保

use fukurow_core::model::{Triple, JsonLdDocument};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod store;
pub mod provenance;
pub mod persistence;
pub mod adapter;

pub use store::*;
pub use provenance::*;
pub use persistence::*;

// Re-export for tests
pub use adapter::sqlite::SqliteAdapter;

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_core::model::Triple;

    #[test]
    fn test_rdf_store_creation() {
        let store = RdfStore::new();
        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);
    }

    #[test]
    fn test_insert_single_triple() {
        let mut store = RdfStore::new();
        let triple = Triple {
            subject: "s1".to_string(),
            predicate: "p1".to_string(),
            object: "o1".to_string(),
        };
        let provenance = Provenance::Sensor {
            source: "test-sensor".to_string(),
            confidence: Some(0.9),
        };

        store.insert(triple.clone(), GraphId::Default, provenance.clone());

        assert_eq!(store.statistics().total_triples, 1);
        assert_eq!(store.statistics().graph_count, 1);

        let results = store.find_triples(Some("s1"), Some("p1"), Some("o1"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].triple, triple);
        assert_eq!(results[0].provenance, provenance);
    }

    #[test]
    fn test_graph_id_display() {
        assert_eq!(format!("{}", GraphId::Default), "default");
        assert_eq!(format!("{}", GraphId::Named("test".to_string())), "named:test");
        assert_eq!(format!("{}", GraphId::Sensor("sensor1".to_string())), "sensor:sensor1");
        assert_eq!(format!("{}", GraphId::Inferred("rule1".to_string())), "inferred:rule1");
    }

    #[test]
    fn test_provenance_sensor() {
        let provenance = Provenance::Sensor {
            source: "test-sensor".to_string(),
            confidence: Some(0.85),
        };

        match provenance {
            Provenance::Sensor { source, confidence } => {
                assert_eq!(source, "test-sensor");
                assert_eq!(confidence, Some(0.85));
            }
            _ => panic!("Expected Sensor provenance"),
        }
    }

    #[test]
    fn test_persistence_backend_memory() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let backend = PersistenceBackend::Memory;
        let pm = PersistenceManager::new(backend);

        let store = RdfStore::new();
        assert_eq!(store.statistics().total_triples, 0);

        // Save should be no-op for memory
        rt.block_on(pm.save_store(&store)).unwrap();

        // Load should return empty store for memory
        let loaded = rt.block_on(pm.load_store()).unwrap();
        assert_eq!(loaded.statistics().total_triples, 0);
    }

    #[test]
    fn test_audit_operation_display() {
        let operation = AuditOperation::Insert {
            triple: "s p o".to_string(),
            graph_id: GraphId::Default,
            provenance: Provenance::Sensor {
                source: "test".to_string(),
                confidence: None,
            },
        };

        match operation {
            AuditOperation::Insert { triple, graph_id, provenance } => {
                assert_eq!(triple, "s p o");
                assert_eq!(graph_id, GraphId::Default);
                match provenance {
                    Provenance::Sensor { source, confidence } => {
                        assert_eq!(source, "test");
                        assert_eq!(confidence, None);
                    }
                    _ => panic!("Expected Sensor provenance"),
                }
            }
            _ => panic!("Expected Insert operation"),
        }
    }

    #[test]
    fn test_find_triples_multiple_patterns() {
        let mut store = RdfStore::new();
        let triple1 = Triple { subject: "s1".to_string(), predicate: "p1".to_string(), object: "o1".to_string() };
        let triple2 = Triple { subject: "s1".to_string(), predicate: "p2".to_string(), object: "o2".to_string() };
        let triple3 = Triple { subject: "s2".to_string(), predicate: "p1".to_string(), object: "o3".to_string() };

        store.insert(triple1.clone(), GraphId::Default, Provenance::Sensor { source: "test".to_string(), confidence: None });
        store.insert(triple2.clone(), GraphId::Default, Provenance::Sensor { source: "test".to_string(), confidence: None });
        store.insert(triple3.clone(), GraphId::Default, Provenance::Sensor { source: "test".to_string(), confidence: None });

        // Test subject + predicate
        let results = store.find_triples(Some("s1"), Some("p1"), None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].triple, triple1);

        // Test predicate + object
        let results = store.find_triples(None, Some("p1"), Some("o3"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].triple, triple3);

        // Test subject only
        let results = store.find_triples(Some("s1"), None, None);
        assert_eq!(results.len(), 2);

        // Test no filter (all triples)
        let results = store.find_triples(None, None, None);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_get_graph() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test_graph".to_string());
        let triple = Triple { subject: "s".to_string(), predicate: "p".to_string(), object: "o".to_string() };

        store.insert(triple.clone(), graph_id.clone(), Provenance::Sensor { source: "test".to_string(), confidence: None });

        let graph = store.get_graph(&graph_id);
        assert_eq!(graph.len(), 1);
        assert_eq!(graph[0].triple, triple);
    }

    #[test]
    fn test_clear_graph() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test_graph".to_string());
        let triple = Triple { subject: "s".to_string(), predicate: "p".to_string(), object: "o".to_string() };

        store.insert(triple, graph_id.clone(), Provenance::Sensor { source: "test".to_string(), confidence: None });
        assert_eq!(store.statistics().total_triples, 1);

        store.clear_graph(&graph_id);
        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);
    }

    #[test]
    fn test_clear_all() {
        let mut store = RdfStore::new();
        let triple1 = Triple { subject: "s1".to_string(), predicate: "p1".to_string(), object: "o1".to_string() };
        let triple2 = Triple { subject: "s2".to_string(), predicate: "p2".to_string(), object: "o2".to_string() };

        store.insert(triple1, GraphId::Default, Provenance::Sensor { source: "test".to_string(), confidence: None });
        store.insert(triple2, GraphId::Named("g1".to_string()), Provenance::Sensor { source: "test".to_string(), confidence: None });
        assert_eq!(store.statistics().total_triples, 2);

        store.clear_all();
        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);
    }

    #[test]
    fn test_audit_trail_limit() {
        let mut store = RdfStore::with_audit_limit(2);

        store.insert(Triple { subject: "s1".to_string(), predicate: "p1".to_string(), object: "o1".to_string() }, GraphId::Default, Provenance::Sensor { source: "test".to_string(), confidence: None });
        store.insert(Triple { subject: "s2".to_string(), predicate: "p2".to_string(), object: "o2".to_string() }, GraphId::Default, Provenance::Sensor { source: "test".to_string(), confidence: None });
        store.insert(Triple { subject: "s3".to_string(), predicate: "p3".to_string(), object: "o3".to_string() }, GraphId::Default, Provenance::Sensor { source: "test".to_string(), confidence: None });

        assert_eq!(store.audit_trail().len(), 2); // Should be limited
    }

    #[test]
    fn test_graph_id_equality() {
        assert_eq!(GraphId::Default, GraphId::Default);
        assert_eq!(GraphId::Named("test".to_string()), GraphId::Named("test".to_string()));
        assert_ne!(GraphId::Default, GraphId::Named("test".to_string()));
    }

    #[test]
    fn test_provenance_equality() {
        let p1 = Provenance::Sensor { source: "test".to_string(), confidence: Some(0.9) };
        let p2 = Provenance::Sensor { source: "test".to_string(), confidence: Some(0.9) };
        let p3 = Provenance::Sensor { source: "other".to_string(), confidence: Some(0.9) };

        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_provenance_inferred() {
        let evidence = vec!["rule1".to_string(), "rule2".to_string()];
        let provenance = Provenance::Inferred {
            rule: "inference_rule".to_string(),
            reasoning_level: "owl".to_string(),
            evidence: evidence.clone(),
        };

        match provenance {
            Provenance::Inferred { rule, reasoning_level, evidence: ev } => {
                assert_eq!(rule, "inference_rule");
                assert_eq!(reasoning_level, "owl");
                assert_eq!(ev, evidence);
            }
            _ => panic!("Expected Inferred provenance"),
        }
    }

    #[test]
    fn test_provenance_imported() {
        use chrono::Utc;
        let imported_at = Utc::now();
        let provenance = Provenance::Imported {
            source_uri: "http://example.com/data.ttl".to_string(),
            imported_at,
        };

        match provenance {
            Provenance::Imported { source_uri, imported_at: at } => {
                assert_eq!(source_uri, "http://example.com/data.ttl");
                assert_eq!(at, imported_at);
            }
            _ => panic!("Expected Imported provenance"),
        }
    }

    #[test]
    fn test_audit_operation_variants() {
        // Test all audit operation variants
        let insert_op = AuditOperation::Insert {
            triple: "s p o".to_string(),
            graph_id: GraphId::Default,
            provenance: Provenance::Sensor { source: "test".to_string(), confidence: None },
        };

        let delete_op = AuditOperation::Delete {
            triple: "s p o".to_string(),
            graph_id: GraphId::Named("test".to_string()),
        };

        let clear_op = AuditOperation::Clear {
            graph_id: GraphId::Sensor("sensor1".to_string()),
            triple_count: 10,
        };

        let inference_op = AuditOperation::Inference {
            rule: "test_rule".to_string(),
            triples_added: 5,
            triples_removed: 2,
        };

        let query_op = AuditOperation::Query {
            query_type: "SPARQL".to_string(),
            result_count: 100,
        };

        // Just ensure they can be created and pattern matched
        match insert_op {
            AuditOperation::Insert { .. } => {}
            _ => panic!("Expected Insert"),
        }

        match delete_op {
            AuditOperation::Delete { .. } => {}
            _ => panic!("Expected Delete"),
        }

        match clear_op {
            AuditOperation::Clear { .. } => {}
            _ => panic!("Expected Clear"),
        }

        match inference_op {
            AuditOperation::Inference { .. } => {}
            _ => panic!("Expected Inference"),
        }

        match query_op {
            AuditOperation::Query { .. } => {}
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_all_triples() {
        let mut store = RdfStore::new();
        let triple1 = Triple { subject: "s1".to_string(), predicate: "p1".to_string(), object: "o1".to_string() };
        let triple2 = Triple { subject: "s2".to_string(), predicate: "p2".to_string(), object: "o2".to_string() };

        store.insert(triple1.clone(), GraphId::Default, Provenance::Sensor { source: "test".to_string(), confidence: None });
        store.insert(triple2.clone(), GraphId::Named("g1".to_string()), Provenance::Sensor { source: "test".to_string(), confidence: None });

        let all_triples = store.all_triples();
        assert_eq!(all_triples.len(), 2);

        let default_triples = all_triples.get(&GraphId::Default).unwrap();
        assert_eq!(default_triples.len(), 1);
        assert_eq!(default_triples[0].triple, triple1);

        let named_triples = all_triples.get(&GraphId::Named("g1".to_string())).unwrap();
        assert_eq!(named_triples.len(), 1);
        assert_eq!(named_triples[0].triple, triple2);
    }

    #[test]
    fn test_statistics() {
        let mut store = RdfStore::new();
        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);

        store.insert(Triple { subject: "s".to_string(), predicate: "p".to_string(), object: "o".to_string() }, GraphId::Default, Provenance::Sensor { source: "test".to_string(), confidence: None });

        let stats = store.statistics();
        assert_eq!(stats.total_triples, 1);
        assert_eq!(stats.graph_count, 1);
    }

    #[tokio::test]
    async fn test_persistence_manager_sqlite_backend_integration() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = format!("sqlite://{}", temp_file.path().to_str().unwrap());

        let backend = PersistenceBackend::Sqlite { url: db_path };
        let pm = PersistenceManager::new(backend);

        // Create test store
        let mut store = RdfStore::new();
        let triple = Triple {
            subject: "http://example.org/test".to_string(),
            predicate: "http://example.org/predicate".to_string(),
            object: "http://example.org/object".to_string(),
        };
        store.insert(triple.clone(), GraphId::Default, Provenance::Sensor {
            source: "test".to_string(),
            confidence: Some(0.9),
        });

        // Save store
        pm.save_store(&store).await.unwrap();

        // Load store
        let loaded = pm.load_store().await.unwrap();

        // Verify
        assert_eq!(loaded.statistics().total_triples, 1);
        assert_eq!(loaded.statistics().graph_count, 1);

        let results = loaded.find_triples(Some("http://example.org/test"), Some("http://example.org/predicate"), Some("http://example.org/object"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].triple, triple);
    }

    #[tokio::test]
    async fn test_persistence_manager_sled_backend_error() {
        let backend = PersistenceBackend::Sled {
            path: "/tmp/test_sled".to_string()
        };
        let pm = PersistenceManager::new(backend);

        let store = RdfStore::new();

        // Save should fail
        let result = pm.save_store(&store).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Sled backend not implemented yet"));

        // Load should fail
        let result = pm.load_store().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Sled backend not implemented yet"));
    }

    #[test]
    fn test_backup_manager() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let backup_manager = BackupManager::new("/tmp/backups".to_string());
        let store = RdfStore::new();

        // Create backup should fail (not implemented)
        let result = rt.block_on(backup_manager.create_backup(&store, "test_backup"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Backup functionality not implemented yet"));

        // Restore backup should fail (not implemented)
        let result = rt.block_on(backup_manager.restore_backup("test_backup"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Restore functionality not implemented yet"));
    }

    #[test]
    fn test_export_to_jsonld() {
        let mut store = RdfStore::new();
        let triple = Triple {
            subject: "http://example.org/subject".to_string(),
            predicate: "http://example.org/predicate".to_string(),
            object: "http://example.org/object".to_string(),
        };
        store.insert(triple.clone(), GraphId::Default, Provenance::Sensor {
            source: "test".to_string(),
            confidence: None,
        });

        let jsonld = export::export_to_jsonld(&store).unwrap();

        assert!(jsonld.graph.is_some());
        let graph = jsonld.graph.as_ref().unwrap();
        assert_eq!(graph.len(), 1);

        let node = &graph[0];
        assert_eq!(node["@id"], "http://example.org/subject");
        assert_eq!(node["http://example.org/predicate"], "http://example.org/object");
    }

    #[test]
    fn test_export_audit_trail() {
        let mut store = RdfStore::new();
        store.insert(Triple {
            subject: "s".to_string(),
            predicate: "p".to_string(),
            object: "o".to_string(),
        }, GraphId::Default, Provenance::Sensor {
            source: "test".to_string(),
            confidence: None,
        });

        let audit_json = export::export_audit_trail(&store).unwrap();
        assert!(audit_json.contains("Insert"));
        assert!(audit_json.contains("test"));
    }

    #[test]
    fn test_export_statistics() {
        let mut store = RdfStore::new();
        store.insert(Triple {
            subject: "s".to_string(),
            predicate: "p".to_string(),
            object: "o".to_string(),
        }, GraphId::Default, Provenance::Sensor {
            source: "test".to_string(),
            confidence: None,
        });

        let stats_json = export::export_statistics(&store).unwrap();
        assert!(stats_json.contains("total_triples"));
        assert!(stats_json.contains("graph_count"));
    }

    #[test]
    fn test_jena_sync() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let jena_sync = sync::JenaSync::new("http://localhost:3030/test".to_string());
        let store = RdfStore::new();

        // Push should fail (not implemented)
        let result = rt.block_on(jena_sync.push_to_jena(&store));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Jena sync not implemented yet"));

        // Pull should fail (not implemented)
        let mut store_mut = RdfStore::new();
        let result = rt.block_on(jena_sync.pull_from_jena(&mut store_mut));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Jena sync not implemented yet"));
    }

    #[test]
    fn test_postgres_sync() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let postgres_sync = sync::PostgresSync::new("postgresql://user:pass@localhost/test".to_string());
        let store = RdfStore::new();

        // Push should fail (not implemented)
        let result = rt.block_on(postgres_sync.push_to_postgres(&store));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("PostgreSQL sync not implemented yet"));
    }
}
