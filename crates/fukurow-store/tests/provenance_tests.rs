use chrono::{DateTime, Utc};

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_provenance_sensor_no_confidence() {
        let provenance = Provenance::Sensor {
            source: "test-sensor".to_string(),
            confidence: None,
        };

        match provenance {
            Provenance::Sensor { source, confidence } => {
                assert_eq!(source, "test-sensor");
                assert_eq!(confidence, None);
            }
            _ => panic!("Expected Sensor provenance"),
        }
    }

    #[test]
    fn test_provenance_inferred() {
        let evidence = vec!["triple1".to_string(), "triple2".to_string()];
        let provenance = Provenance::Inferred {
            rule: "test-rule".to_string(),
            reasoning_level: "rdfs".to_string(),
            evidence: evidence.clone(),
        };

        match provenance {
            Provenance::Inferred { rule, reasoning_level, evidence: ev } => {
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
        let provenance = Provenance::Imported {
            source_uri: "file://test.ttl".to_string(),
            imported_at,
        };

        match provenance {
            Provenance::Imported { source_uri, imported_at: at } => {
                assert_eq!(source_uri, "file://test.ttl");
                assert_eq!(at, imported_at);
            }
            _ => panic!("Expected Imported provenance"),
        }
    }

    #[test]
    fn test_graph_id_default() {
        let graph_id = GraphId::Default;
        assert_eq!(format!("{}", graph_id), "default");
    }

    #[test]
    fn test_graph_id_named() {
        let graph_id = GraphId::Named("test_graph".to_string());
        assert_eq!(format!("{}", graph_id), "named:test_graph");
    }

    #[test]
    fn test_graph_id_sensor() {
        let graph_id = GraphId::Sensor("sensor_001".to_string());
        assert_eq!(format!("{}", graph_id), "sensor:sensor_001");
    }

    #[test]
    fn test_graph_id_inferred() {
        let graph_id = GraphId::Inferred("rule_001".to_string());
        assert_eq!(format!("{}", graph_id), "inferred:rule_001");
    }

    #[test]
    fn test_graph_id_default_equality() {
        assert_eq!(GraphId::Default, GraphId::Default);
        assert_ne!(GraphId::Default, GraphId::Named("test".to_string()));
    }

    #[test]
    fn test_graph_id_named_equality() {
        let g1 = GraphId::Named("test".to_string());
        let g2 = GraphId::Named("test".to_string());
        let g3 = GraphId::Named("other".to_string());

        assert_eq!(g1, g2);
        assert_ne!(g1, g3);
    }

    #[test]
    fn test_audit_entry_insert() {
        let timestamp = Utc::now();
        let triple_str = "s p o";
        let graph_id = GraphId::Default;
        let provenance = Provenance::Sensor {
            source: "test".to_string(),
            confidence: None,
        };

        let operation = AuditOperation::Insert {
            triple: triple_str.to_string(),
            graph_id: graph_id.clone(),
            provenance: provenance.clone(),
        };

        let entry = AuditEntry {
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
            AuditOperation::Insert { triple, graph_id: gid, provenance: prov } => {
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
        let graph_id = GraphId::Named("test".to_string());

        let operation = AuditOperation::Delete {
            triple: triple_str.to_string(),
            graph_id: graph_id.clone(),
        };

        let entry = AuditEntry {
            id: "test-id".to_string(),
            timestamp,
            operation,
            actor: None,
            metadata: std::collections::HashMap::new(),
        };

        match entry.operation {
            AuditOperation::Delete { triple, graph_id: gid } => {
                assert_eq!(triple, triple_str);
                assert_eq!(gid, graph_id);
            }
            _ => panic!("Expected Delete operation"),
        }
    }

    #[test]
    fn test_audit_entry_clear() {
        let timestamp = Utc::now();
        let graph_id = GraphId::Sensor("sensor1".to_string());

        let operation = AuditOperation::Clear {
            graph_id: graph_id.clone(),
            triple_count: 42,
        };

        let entry = AuditEntry {
            id: "test-id".to_string(),
            timestamp,
            operation,
            actor: None,
            metadata: std::collections::HashMap::new(),
        };

        match entry.operation {
            AuditOperation::Clear { graph_id: gid, triple_count } => {
                assert_eq!(gid, graph_id);
                assert_eq!(triple_count, 42);
            }
            _ => panic!("Expected Clear operation"),
        }
    }

    #[test]
    fn test_audit_entry_inference() {
        let timestamp = Utc::now();

        let operation = AuditOperation::Inference {
            rule: "test-rule".to_string(),
            triples_added: 10,
            triples_removed: 2,
        };

        let entry = AuditEntry {
            id: "test-id".to_string(),
            timestamp,
            operation,
            actor: None,
            metadata: std::collections::HashMap::new(),
        };

        match entry.operation {
            AuditOperation::Inference { rule, triples_added, triples_removed } => {
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

        let operation = AuditOperation::Query {
            query_type: "SPARQL".to_string(),
            result_count: 100,
        };

        let entry = AuditEntry {
            id: "test-id".to_string(),
            timestamp,
            operation,
            actor: None,
            metadata: std::collections::HashMap::new(),
        };

        match entry.operation {
            AuditOperation::Query { query_type, result_count } => {
                assert_eq!(query_type, "SPARQL");
                assert_eq!(result_count, 100);
            }
            _ => panic!("Expected Query operation"),
        }
    }
}
