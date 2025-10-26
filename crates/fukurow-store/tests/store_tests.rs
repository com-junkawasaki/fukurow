use fukurow_core::model::Triple;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_triple(subject: &str, predicate: &str, object: &str) -> Triple {
        Triple {
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
        }
    }

    fn create_test_provenance() -> crate::Provenance {
        crate::Provenance::Sensor {
            source: "test-sensor".to_string(),
            confidence: Some(0.9),
        }
    }

    #[test]
    fn test_rdf_store_creation() {
        let store = crate::RdfStore::new();
        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);
    }

    #[test]
    fn test_rdf_store_with_audit_limit() {
        let store = RdfStore::with_audit_limit(50);
        assert_eq!(store.audit_trail().len(), 0);
    }

    #[test]
    fn test_insert_single_triple() {
        let mut store = RdfStore::new();
        let triple = create_test_triple("s1", "p1", "o1");
        let provenance = create_test_provenance();

        store.insert(triple.clone(), GraphId::Default, provenance.clone());

        assert_eq!(store.statistics().total_triples, 1);
        assert_eq!(store.statistics().graph_count, 1);

        let results = store.find_triples(Some("s1"), Some("p1"), Some("o1"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].triple, triple);
        assert_eq!(results[0].provenance, provenance);
    }

    #[test]
    fn test_insert_multiple_graphs() {
        let mut store = RdfStore::new();

        // Insert to default graph
        let triple1 = create_test_triple("s1", "p1", "o1");
        store.insert(triple1, GraphId::Default, create_test_provenance());

        // Insert to named graph
        let triple2 = create_test_triple("s2", "p2", "o2");
        store.insert(triple2, GraphId::Named("graph1".to_string()), create_test_provenance());

        // Insert to sensor graph
        let triple3 = create_test_triple("s3", "p3", "o3");
        store.insert(triple3, GraphId::Sensor("sensor1".to_string()), create_test_provenance());

        // Insert to inferred graph
        let triple4 = create_test_triple("s4", "p4", "o4");
        store.insert(triple4, GraphId::Inferred("rule1".to_string()), create_test_provenance());

        assert_eq!(store.statistics().total_triples, 4);
        assert_eq!(store.statistics().graph_count, 4);

        let graph_ids = store.graph_ids();
        assert_eq!(graph_ids.len(), 4);
    }

    #[test]
    fn test_find_triples_subject_only() {
        let mut store = RdfStore::new();
        let triple1 = create_test_triple("s1", "p1", "o1");
        let triple2 = create_test_triple("s1", "p2", "o2");
        let triple3 = create_test_triple("s2", "p1", "o1");

        store.insert(triple1.clone(), GraphId::Default, create_test_provenance());
        store.insert(triple2.clone(), GraphId::Default, create_test_provenance());
        store.insert(triple3, GraphId::Default, create_test_provenance());

        let results = store.find_triples(Some("s1"), None, None);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.triple.predicate == "p1"));
        assert!(results.iter().any(|r| r.triple.predicate == "p2"));
    }

    #[test]
    fn test_find_triples_predicate_only() {
        let mut store = RdfStore::new();
        let triple1 = create_test_triple("s1", "p1", "o1");
        let triple2 = create_test_triple("s2", "p1", "o2");
        let triple3 = create_test_triple("s1", "p2", "o1");

        store.insert(triple1.clone(), GraphId::Default, create_test_provenance());
        store.insert(triple2.clone(), GraphId::Default, create_test_provenance());
        store.insert(triple3, GraphId::Default, create_test_provenance());

        let results = store.find_triples(None, Some("p1"), None);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.triple.subject == "s1"));
        assert!(results.iter().any(|r| r.triple.subject == "s2"));
    }

    #[test]
    fn test_find_triples_object_only() {
        let mut store = RdfStore::new();
        let triple1 = create_test_triple("s1", "p1", "o1");
        let triple2 = create_test_triple("s2", "p2", "o1");
        let triple3 = create_test_triple("s1", "p2", "o2");

        store.insert(triple1.clone(), GraphId::Default, create_test_provenance());
        store.insert(triple2.clone(), GraphId::Default, create_test_provenance());
        store.insert(triple3, GraphId::Default, create_test_provenance());

        let results = store.find_triples(None, None, Some("o1"));
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.triple.subject == "s1"));
        assert!(results.iter().any(|r| r.triple.subject == "s2"));
    }

    #[test]
    fn test_find_triples_no_filter() {
        let mut store = RdfStore::new();
        store.insert(create_test_triple("s1", "p1", "o1"), GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), GraphId::Default, create_test_provenance());

        let results = store.find_triples(None, None, None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_get_graph() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test_graph".to_string());

        store.insert(create_test_triple("s1", "p1", "o1"), graph_id.clone(), create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), graph_id.clone(), create_test_provenance());

        let graph = store.get_graph(&graph_id);
        assert_eq!(graph.len(), 2);

        let non_existent = store.get_graph(&GraphId::Named("non_existent".to_string()));
        assert_eq!(non_existent.len(), 0);
    }

    #[test]
    fn test_get_default_graph() {
        let mut store = RdfStore::new();
        store.insert(create_test_triple("s1", "p1", "o1"), GraphId::Default, create_test_provenance());

        let default_graph = store.get_graph(&GraphId::Default);
        assert_eq!(default_graph.len(), 1);
    }

    #[test]
    fn test_clear_graph() {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("test_graph".to_string());

        store.insert(create_test_triple("s1", "p1", "o1"), graph_id.clone(), create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), graph_id.clone(), create_test_provenance());

        assert_eq!(store.statistics().total_triples, 2);
        assert_eq!(store.statistics().graph_count, 1);

        store.clear_graph(&graph_id);

        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);
        assert_eq!(store.audit_trail().len(), 1);
    }

    #[test]
    fn test_clear_all() {
        let mut store = RdfStore::new();

        store.insert(create_test_triple("s1", "p1", "o1"), GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), GraphId::Named("g1".to_string()), create_test_provenance());

        assert_eq!(store.statistics().total_triples, 2);
        assert_eq!(store.statistics().graph_count, 2);

        store.clear_all();

        assert_eq!(store.statistics().total_triples, 0);
        assert_eq!(store.statistics().graph_count, 0);
        assert_eq!(store.audit_trail().len(), 1);
    }

    #[test]
    fn test_audit_trail_limit() {
        let mut store = RdfStore::with_audit_limit(2);

        store.insert(create_test_triple("s1", "p1", "o1"), GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s3", "p3", "o3"), GraphId::Default, create_test_provenance());

        assert_eq!(store.audit_trail().len(), 2); // Should be limited to 2
    }

    #[test]
    fn test_set_audit_limit() {
        let mut store = RdfStore::new();

        store.insert(create_test_triple("s1", "p1", "o1"), GraphId::Default, create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), GraphId::Default, create_test_provenance());

        assert_eq!(store.audit_trail().len(), 2);

        store.set_audit_limit(1);
        assert_eq!(store.audit_trail().len(), 1);
    }

    #[test]
    fn test_all_triples() {
        let mut store = RdfStore::new();
        let graph_id1 = GraphId::Default;
        let graph_id2 = GraphId::Named("g1".to_string());

        store.insert(create_test_triple("s1", "p1", "o1"), graph_id1.clone(), create_test_provenance());
        store.insert(create_test_triple("s2", "p2", "o2"), graph_id2.clone(), create_test_provenance());

        let all_triples = store.all_triples();
        assert_eq!(all_triples.len(), 2);
        assert!(all_triples.contains_key(&graph_id1));
        assert!(all_triples.contains_key(&graph_id2));
    }
}
