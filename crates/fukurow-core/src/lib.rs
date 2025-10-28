//! # Reasoner Graph Library
//!
//! JSON-LDベースのRDFグラフ操作ライブラリ
//! サイバーセキュリティイベントの推論に必要なグラフ構造を提供

pub mod model;
pub mod store;
pub mod query;
pub mod jsonld;

pub use model::*;
pub use store::*;
pub use query::*;
pub use jsonld::*;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[cfg(test)]
    mod interned_string_tests {
        use super::*;

        #[test]
        fn test_interned_string_new() {
            let s1 = InternedString::new("test");
            let s2 = InternedString::new("test");
            assert_eq!(s1.as_str(), "test");
            assert_eq!(s2.as_str(), "test");
            // Same strings should be equal
            assert_eq!(s1, s2);
        }

        #[test]
        fn test_interned_string_len() {
            let s = InternedString::new("hello");
            assert_eq!(s.len(), 5);
        }

        #[test]
        fn test_interned_string_is_empty() {
            let s1 = InternedString::new("");
            let s2 = InternedString::new("not empty");
            assert!(s1.is_empty());
            assert!(!s2.is_empty());
        }

        #[test]
        fn test_interned_string_from_string() {
            let s: InternedString = "test".to_string().into();
            assert_eq!(s.as_str(), "test");
        }

        #[test]
        fn test_interned_string_from_str() {
            let s: InternedString = "test".into();
            assert_eq!(s.as_str(), "test");
        }

        #[test]
        fn test_interned_string_from_string_ref() {
            let string = "test".to_string();
            let s: InternedString = (&string).into();
            assert_eq!(s.as_str(), "test");
        }

        #[test]
        fn test_interned_string_display() {
            let s = InternedString::new("test");
            assert_eq!(format!("{}", s), "test");
        }

        #[test]
        fn test_interned_string_as_ref() {
            let s = InternedString::new("test");
            let str_ref: &str = s.as_ref();
            assert_eq!(str_ref, "test");
        }
    }

    #[cfg(test)]
    mod triple_tests {
        use super::*;

        #[test]
        fn test_triple_creation() {
            let triple = Triple {
                subject: "subject".to_string(),
                predicate: "predicate".to_string(),
                object: "object".to_string(),
            };
            assert_eq!(triple.subject, "subject");
            assert_eq!(triple.predicate, "predicate");
            assert_eq!(triple.object, "object");
        }

        #[test]
        fn test_triple_equality() {
            let triple1 = Triple {
                subject: "s".to_string(),
                predicate: "p".to_string(),
                object: "o".to_string(),
            };
            let triple2 = Triple {
                subject: "s".to_string(),
                predicate: "p".to_string(),
                object: "o".to_string(),
            };
            let triple3 = Triple {
                subject: "s".to_string(),
                predicate: "p".to_string(),
                object: "different".to_string(),
            };
            assert_eq!(triple1, triple2);
            assert_ne!(triple1, triple3);
        }
    }

    #[cfg(test)]
    mod interned_triple_tests {
        use super::*;

        #[test]
        fn test_interned_triple_new() {
            let triple = InternedTriple::new("subject", "predicate", "object");
            assert_eq!(triple.subject.as_str(), "subject");
            assert_eq!(triple.predicate.as_str(), "predicate");
            assert_eq!(triple.object.as_str(), "object");
        }

        #[test]
        fn test_interned_triple_to_triple() {
            let interned = InternedTriple::new("s", "p", "o");
            let triple = interned.to_triple();
            assert_eq!(triple.subject, "s");
            assert_eq!(triple.predicate, "p");
            assert_eq!(triple.object, "o");
        }

        #[test]
        fn test_interned_triple_from_triple() {
            let triple = Triple {
                subject: "subject".to_string(),
                predicate: "predicate".to_string(),
                object: "object".to_string(),
            };
            let interned = InternedTriple::from_triple(&triple);
            assert_eq!(interned.subject.as_str(), "subject");
            assert_eq!(interned.predicate.as_str(), "predicate");
            assert_eq!(interned.object.as_str(), "object");
        }

        #[test]
        fn test_interned_triple_equality() {
            let triple1 = InternedTriple::new("s", "p", "o");
            let triple2 = InternedTriple::new("s", "p", "o");
            let triple3 = InternedTriple::new("s", "p", "different");
            assert_eq!(triple1, triple2);
            assert_ne!(triple1, triple3);
        }
    }

    #[cfg(test)]
    mod jsonld_tests {
        use super::*;

        #[test]
        fn test_jsonld_to_triples_simple() {
            let jsonld = JsonLdDocument {
                context: serde_json::json!({"@vocab": "https://example.org/"}),
                graph: Some(vec![
                    serde_json::json!({
                        "@id": "subject1",
                        "predicate1": "object1",
                        "predicate2": "object2"
                    })
                ]),
                data: std::collections::HashMap::new(),
            };

            let triples = jsonld_to_triples(&jsonld).unwrap();
            assert_eq!(triples.len(), 2);

            assert!(triples.contains(&Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            }));

            assert!(triples.contains(&Triple {
                subject: "subject1".to_string(),
                predicate: "predicate2".to_string(),
                object: "object2".to_string(),
            }));
        }

        #[test]
        fn test_jsonld_to_triples_empty() {
            let jsonld = JsonLdDocument {
                context: serde_json::json!({}),
                graph: Some(vec![]),
                data: std::collections::HashMap::new(),
            };

            let triples = jsonld_to_triples(&jsonld).unwrap();
            assert_eq!(triples.len(), 0);
        }

        #[test]
        fn test_cyber_event_to_jsonld_network_connection() {
            let event = CyberEvent::NetworkConnection {
                source_ip: "192.168.1.1".to_string(),
                dest_ip: "10.0.0.1".to_string(),
                port: 443,
                protocol: "tcp".to_string(),
                timestamp: 1640995200,
            };

            let jsonld = cyber_event_to_jsonld(&event).unwrap();

            assert!(jsonld.graph.is_some());
            let graph = jsonld.graph.as_ref().unwrap();
            assert_eq!(graph.len(), 1);

            let node = &graph[0];
            assert_eq!(node.get("@type").unwrap(), "NetworkConnection");
            assert_eq!(node.get("sourceIp").unwrap(), "192.168.1.1");
            assert_eq!(node.get("destIp").unwrap(), "10.0.0.1");
            assert_eq!(node.get("port").unwrap(), 443);
            assert_eq!(node.get("protocol").unwrap(), "tcp");
            assert_eq!(node.get("timestamp").unwrap(), 1640995200);
        }

        #[test]
        fn test_cyber_event_to_jsonld_process_execution() {
            let event = CyberEvent::ProcessExecution {
                process_id: 1234,
                parent_process_id: Some(5678),
                command_line: "/bin/bash".to_string(),
                user: "alice".to_string(),
                timestamp: 1640995200,
            };

            let jsonld = cyber_event_to_jsonld(&event).unwrap();
            let graph = jsonld.graph.as_ref().unwrap();
            let node = &graph[0];

            assert_eq!(node.get("@type").unwrap(), "ProcessExecution");
            assert_eq!(node.get("processId").unwrap(), 1234);
            assert_eq!(node.get("parentProcessId").unwrap(), 5678);
            assert_eq!(node.get("commandLine").unwrap(), "/bin/bash");
            assert_eq!(node.get("user").unwrap(), "alice");
        }

        #[test]
        fn test_cyber_event_to_jsonld_file_access() {
            let event = CyberEvent::FileAccess {
                file_path: "/etc/passwd".to_string(),
                access_type: "read".to_string(),
                user: "bob".to_string(),
                process_id: 9999,
                timestamp: 1640995200,
            };

            let jsonld = cyber_event_to_jsonld(&event).unwrap();
            let graph = jsonld.graph.as_ref().unwrap();
            let node = &graph[0];

            assert_eq!(node.get("@type").unwrap(), "FileAccess");
            assert_eq!(node.get("filePath").unwrap(), "/etc/passwd");
            assert_eq!(node.get("accessType").unwrap(), "read");
            assert_eq!(node.get("user").unwrap(), "bob");
            assert_eq!(node.get("processId").unwrap(), 9999);
        }

        #[test]
        fn test_cyber_event_to_jsonld_user_login() {
            let event = CyberEvent::UserLogin {
                user: "charlie".to_string(),
                source_ip: "203.0.113.1".to_string(),
                success: true,
                timestamp: 1640995200,
            };

            let jsonld = cyber_event_to_jsonld(&event).unwrap();
            let graph = jsonld.graph.as_ref().unwrap();
            let node = &graph[0];

            assert_eq!(node.get("@type").unwrap(), "UserLogin");
            assert_eq!(node.get("user").unwrap(), "charlie");
            assert_eq!(node.get("sourceIp").unwrap(), "203.0.113.1");
            assert_eq!(node.get("success").unwrap(), true);
        }

        #[test]
        fn test_parse_jsonld() {
            let json_str = r#"{
                "@context": {"@vocab": "https://example.org/"},
                "@graph": [
                    {
                        "@id": "subject1",
                        "predicate1": "object1"
                    }
                ]
            }"#;

            let doc = parse_jsonld(json_str).unwrap();
            assert!(doc.graph.is_some());
            assert_eq!(doc.graph.as_ref().unwrap().len(), 1);
        }

        #[test]
        fn test_serialize_jsonld() {
            let doc = JsonLdDocument {
                context: serde_json::json!({"@vocab": "https://example.org/"}),
                graph: Some(vec![
                    serde_json::json!({
                        "@id": "subject1",
                        "predicate1": "object1"
                    })
                ]),
                data: std::collections::HashMap::new(),
            };

            let json_str = serialize_jsonld(&doc).unwrap();
            assert!(json_str.contains("@context"));
            assert!(json_str.contains("@graph"));
            assert!(json_str.contains("subject1"));
        }
    }

    #[cfg(test)]
    mod store_tests {
        use super::*;

        #[test]
        fn test_graph_store_new() {
            let store = GraphStore::new();
            assert_eq!(store.graph_names().len(), 0);
            assert_eq!(store.get_default_graph().triples.len(), 0);
        }

        #[test]
        fn test_add_triple_to_default() {
            let mut store = GraphStore::new();
            let triple = Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            };

            store.add_triple(triple.clone());
            assert_eq!(store.get_default_graph().triples.len(), 1);
            assert_eq!(store.get_default_graph().triples[0], triple);
        }

        #[test]
        fn test_add_triple_to_named_graph() {
            let mut store = GraphStore::new();
            let triple = Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            };

            store.add_triple_to_graph("test_graph", triple.clone());

            let graph = store.get_graph("test_graph").unwrap();
            assert_eq!(graph.triples.len(), 1);
            assert_eq!(graph.triples[0], triple);
            assert_eq!(graph.name, "test_graph");
        }

        #[test]
        fn test_add_triples_to_graph() {
            let mut store = GraphStore::new();
            let triples = vec![
                Triple {
                    subject: "subject1".to_string(),
                    predicate: "predicate1".to_string(),
                    object: "object1".to_string(),
                },
                Triple {
                    subject: "subject2".to_string(),
                    predicate: "predicate2".to_string(),
                    object: "object2".to_string(),
                },
            ];

            store.add_triples_to_graph("test_graph", triples.clone());

            let graph = store.get_graph("test_graph").unwrap();
            assert_eq!(graph.triples.len(), 2);
            assert_eq!(graph.triples, triples);
        }

        #[test]
        fn test_graph_names() {
            let mut store = GraphStore::new();

            store.add_triple_to_graph("graph1", Triple {
                subject: "s".to_string(),
                predicate: "p".to_string(),
                object: "o".to_string(),
            });

            store.add_triple_to_graph("graph2", Triple {
                subject: "s".to_string(),
                predicate: "p".to_string(),
                object: "o".to_string(),
            });

            let names = store.graph_names();
            assert_eq!(names.len(), 2);
            assert!(names.contains(&"graph1".to_string()));
            assert!(names.contains(&"graph2".to_string()));
        }

        #[test]
        fn test_find_triples_exact_match() {
            let mut store = GraphStore::new();

            let triple1 = Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            };

            let triple2 = Triple {
                subject: "subject1".to_string(),
                predicate: "predicate2".to_string(),
                object: "object2".to_string(),
            };

            store.add_triple(triple1.clone());
            store.add_triple_to_graph("named", triple2.clone());

            let results = store.find_triples(Some("subject1"), Some("predicate1"), Some("object1"));
            assert_eq!(results.len(), 1);
            assert_eq!(results[0], &triple1);
        }

        #[test]
        fn test_find_triples_partial_match() {
            let mut store = GraphStore::new();

            let triple1 = Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            };

            let triple2 = Triple {
                subject: "subject1".to_string(),
                predicate: "predicate2".to_string(),
                object: "object2".to_string(),
            };

            store.add_triple(triple1.clone());
            store.add_triple(triple2.clone());

            // Find by subject only
            let results = store.find_triples(Some("subject1"), None, None);
            assert_eq!(results.len(), 2);
            assert!(results.contains(&&triple1));
            assert!(results.contains(&&triple2));
        }

        #[test]
        fn test_find_triples_no_match() {
            let mut store = GraphStore::new();

            store.add_triple(Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            });

            let results = store.find_triples(Some("nonexistent"), None, None);
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_to_jsonld() {
            let mut store = GraphStore::new();

            store.add_triple(Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            });

            store.add_triple_to_graph("named", Triple {
                subject: "subject2".to_string(),
                predicate: "predicate2".to_string(),
                object: "object2".to_string(),
            });

            let jsonld = store.to_jsonld().unwrap();
            assert!(jsonld.graph.is_some());
            assert_eq!(jsonld.graph.as_ref().unwrap().len(), 2);
        }

        #[test]
        fn test_clear() {
            let mut store = GraphStore::new();

            store.add_triple(Triple {
                subject: "s".to_string(),
                predicate: "p".to_string(),
                object: "o".to_string(),
            });

            store.add_triple_to_graph("named", Triple {
                subject: "s".to_string(),
                predicate: "p".to_string(),
                object: "o".to_string(),
            });

            assert_eq!(store.get_default_graph().triples.len(), 1);
            assert!(store.get_graph("named").is_some());

            store.clear();

            assert_eq!(store.get_default_graph().triples.len(), 0);
            assert!(store.get_graph("named").is_none());
            assert_eq!(store.graph_names().len(), 0);
        }

        #[test]
        fn test_graph_store_to_jsonld() {
            let mut store = GraphStore::new();

            store.add_triple(Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            });

            let jsonld = store.to_jsonld().unwrap();
            assert!(jsonld.graph.is_some());
            assert_eq!(jsonld.graph.as_ref().unwrap().len(), 1);
        }
    }

    #[cfg(test)]
    mod query_tests {
        use super::*;
        use crate::query::{GraphQuery, var, const_val};

        #[test]
        fn test_graph_query_new() {
            let query = GraphQuery::new();
            assert_eq!(query.pattern_count(), 0);
        }

        #[test]
        fn test_graph_query_where_clause() {
            let query = GraphQuery::new()
                .where_clause(
                    var("s"),
                    const_val("predicate1"),
                    var("o")
                );

            assert_eq!(query.pattern_count(), 1);
        }

        #[test]
        fn test_graph_query_execute_simple() {
            let mut store = GraphStore::new();

            store.add_triple(Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            });

            let query = GraphQuery::new()
                .where_clause(
                    var("s"),
                    const_val("predicate1"),
                    var("o")
                );

            let results = query.execute(&store);
            assert_eq!(results.len(), 1);

            let binding = &results[0];
            assert_eq!(binding.get("s"), Some(&"subject1".to_string()));
            assert_eq!(binding.get("o"), Some(&"object1".to_string()));
        }

        #[test]
        fn test_graph_query_execute_multiple_patterns() {
            let mut store = GraphStore::new();

            store.add_triple(Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            });

            store.add_triple(Triple {
                subject: "subject1".to_string(),
                predicate: "predicate2".to_string(),
                object: "object2".to_string(),
            });

            let query = GraphQuery::new()
                .where_clause(
                    var("s"),
                    const_val("predicate1"),
                    var("o1")
                )
                .where_clause(
                    var("s"),
                    const_val("predicate2"),
                    var("o2")
                );

            let results = query.execute(&store);
            assert_eq!(results.len(), 1);

            let binding = &results[0];
            assert_eq!(binding.get("s"), Some(&"subject1".to_string()));
            assert_eq!(binding.get("o1"), Some(&"object1".to_string()));
            assert_eq!(binding.get("o2"), Some(&"object2".to_string()));
        }

        #[test]
        fn test_graph_query_execute_no_match() {
            let mut store = GraphStore::new();

            store.add_triple(Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            });

            let query = GraphQuery::new()
                .where_clause(
                    var("s"),
                    const_val("nonexistent"),
                    var("o")
                );

            let results = query.execute(&store);
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_graph_query_execute_with_constants() {
            let mut store = GraphStore::new();

            store.add_triple(Triple {
                subject: "subject1".to_string(),
                predicate: "predicate1".to_string(),
                object: "object1".to_string(),
            });

            let query = GraphQuery::new()
                .where_clause(
                    const_val("subject1"),
                    const_val("predicate1"),
                    const_val("object1")
                );

            let results = query.execute(&store);
            assert_eq!(results.len(), 1);

            let binding = &results[0];
            assert_eq!(binding.len(), 0); // No variables to bind
        }

        #[test]
        fn test_graph_query_execute_multiple_results() {
            let mut store = GraphStore::new();

            store.add_triple(Triple {
                subject: "subject1".to_string(),
                predicate: "type".to_string(),
                object: "Person".to_string(),
            });

            store.add_triple(Triple {
                subject: "subject2".to_string(),
                predicate: "type".to_string(),
                object: "Person".to_string(),
            });

            let query = GraphQuery::new()
                .where_clause(
                    var("person"),
                    const_val("type"),
                    const_val("Person")
                );

            let results = query.execute(&store);
            assert_eq!(results.len(), 2);

            let persons: std::collections::HashSet<_> = results.iter()
                .filter_map(|r| r.get("person"))
                .collect();

            assert!(persons.contains(&"subject1".to_string()));
            assert!(persons.contains(&"subject2".to_string()));
        }

        #[test]
        fn test_helper_functions() {
            let var_pattern = var("test_var");
            match var_pattern {
                PatternValue::Variable(v) => assert_eq!(v, "test_var"),
                _ => panic!("Expected variable"),
            }

            let const_pattern = const_val("test_const");
            match const_pattern {
                PatternValue::Constant(c) => assert_eq!(c, "test_const"),
                _ => panic!("Expected constant"),
            }
        }

    }
}
