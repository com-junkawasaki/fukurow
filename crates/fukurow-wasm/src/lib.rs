//! Headless WebAssembly bindings for Fukurow reasoning engine
//!
//! DOM/Node 依存を一切持たない、純計算 API を提供します。

use wasm_bindgen::prelude::*;
use serde::Deserialize;
use fukurow_lite::{RdfStore, Provenance, GraphId, Triple, OwlLiteReasoner, OntologyLoader};
use fukurow_lite::model::{Ontology, Class, Axiom, OwlIri};
use fukurow_lite::loader::DefaultOntologyLoader;
use fukurow_sparql::QueryResult as SparqlResult;
use fukurow_shacl::{ShaclLoader, ValidationReport};
use fukurow_shacl::loader::DefaultShaclLoader;
use fukurow_shacl::validator::{ShaclValidator, DefaultShaclValidator, ValidationConfig};

#[derive(Debug, Deserialize)]
struct ReasonOptions {
    // "lite" | "dl"
    #[serde(default = "default_engine")]
    engine: String,
    // 推論モード/制約など、将来拡張用
    #[serde(default)]
    params: serde_json::Value,
}
fn default_engine() -> String { "lite".to_string() }

// Simplified JSON-LD processing for WASM
fn jsonld_to_store(jsonld_str: &str) -> Result<RdfStore, JsValue> {
    // For simplicity, parse basic JSON-LD format manually
    let json: serde_json::Value = serde_json::from_str(jsonld_str)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;

    let mut store = RdfStore::new();
    let graph_id = GraphId::Default;
    let provenance = Provenance::Sensor {
        source: "wasm-input".to_string(),
        confidence: Some(1.0),
    };

    // Extract triples from basic JSON-LD structure
    if let Some(graph) = json.get("@graph") {
        if let Some(graph_array) = graph.as_array() {
            for node in graph_array {
                if let Some(node_obj) = node.as_object() {
                    if let Some(subject) = node_obj.get("@id") {
                        let subject_str = subject.as_str().unwrap_or("");

                        for (key, value) in node_obj {
                            if key != "@id" && key != "@type" {
                                if let Some(obj_str) = value.as_str() {
                                    // Use the Triple type from fukurow-core
                                    let triple = Triple {
                                        subject: subject_str.to_string(),
                                        predicate: key.clone(),
                                        object: obj_str.to_string(),
                                    };
                                    store.insert(triple, graph_id.clone(), provenance.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(store)
}

fn store_to_jsonld(store: &RdfStore) -> Result<String, JsValue> {
    // Simple JSON-LD output
    let mut graph = Vec::new();

    for stored_triple in store.all_triples().values().flatten() {
        let triple = &stored_triple.triple;

        let mut node = serde_json::Map::new();
        node.insert("@id".to_string(), serde_json::Value::String(triple.subject.clone()));

        let mut properties = serde_json::Map::new();
        properties.insert(triple.predicate.clone(), serde_json::Value::String(triple.object.clone()));
        node.insert("properties".to_string(), serde_json::Value::Object(properties));

        graph.push(serde_json::Value::Object(node));
    }

    let result = serde_json::json!({
        "@context": { "@vocab": "http://www.w3.org/2002/07/owl#" },
        "@graph": graph
    });

    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialize error: {}", e)))
}

#[wasm_bindgen]
pub fn reason_owl(input_jsonld: &str, options_json: &str) -> Result<String, JsValue> {
    let opts: ReasonOptions = serde_json::from_str(options_json).unwrap_or(ReasonOptions {
        engine: default_engine(),
        params: serde_json::json!({}),
    });

    // Parse JSON-LD to RdfStore
    let store = jsonld_to_store(input_jsonld)?;

    // Load ontology from store
    let loader = DefaultOntologyLoader;
    let ontology = loader.load_from_store(&store)
        .map_err(|e| JsValue::from_str(&format!("Ontology loading error: {:?}", e)))?;

    // Create reasoner and perform inference
    let mut reasoner = OwlLiteReasoner::new();

    // Compute class hierarchy (main inference)
    let hierarchy = reasoner.compute_class_hierarchy(&ontology)
        .map_err(|e| JsValue::from_str(&format!("Reasoning error: {:?}", e)))?;

    // Get inferred axioms from hierarchy
    let inferred = reasoner.get_inferred_axioms(&ontology)
        .map_err(|e| JsValue::from_str(&format!("Inference error: {:?}", e)))?;

    // Create result store with original data + inferred axioms
    let mut result_store = RdfStore::new();
    // Copy original triples to result store
    for (graph_id, triples) in store.all_triples() {
        for stored_triple in triples {
            result_store.insert(
                stored_triple.triple.clone(),
                graph_id.clone(),
                stored_triple.provenance.clone(),
            );
        }
    }

    let inferred_graph_id = GraphId::Inferred("owl-reasoning".to_string());
    let inferred_provenance = Provenance::Sensor {
        source: "fukurow-lite".to_string(),
        confidence: Some(1.0),
    };

    // Convert inferred axioms back to triples and add to result store
    for axiom in inferred {
        match axiom {
            fukurow_lite::model::Axiom::SubClassOf(subclass, superclass) => {
                let subject = match subclass {
                    fukurow_lite::model::Class::Named(iri) => iri.0,
                    _ => continue, // Skip non-named classes for now
                };
                let object = match superclass {
                    fukurow_lite::model::Class::Named(iri) => iri.0,
                    _ => continue,
                };

                let triple = Triple {
                    subject,
                    predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                    object,
                };
                result_store.insert(triple, inferred_graph_id.clone(), inferred_provenance.clone());
            }
            // Add other axiom types as needed
            _ => {} // Skip other axiom types for now
        }
    }

    // Serialize result back to JSON-LD
    store_to_jsonld(&result_store)
}

#[wasm_bindgen]
pub fn validate_shacl(data_jsonld: &str, shape_jsonld: &str) -> Result<String, JsValue> {
    // Parse data JSON-LD to RdfStore
    let data_store = jsonld_to_store(data_jsonld)?;

    // Parse shapes JSON-LD to RdfStore
    let shapes_store = jsonld_to_store(shape_jsonld)?;

    // Load shapes graph
    let loader = DefaultShaclLoader;
    let shapes_graph = loader.load_from_store(&shapes_store)
        .map_err(|e| JsValue::from_str(&format!("SHACL loader error: {:?}", e)))?;

    // Create validator and validate
    let validator = DefaultShaclValidator;
    let config = ValidationConfig::default();
    let report = validator.validate_graph(&shapes_graph, &data_store, &config)
        .map_err(|e| JsValue::from_str(&format!("SHACL validation error: {:?}", e)))?;

    // Convert report to JSON - simplified version to avoid serialization issues
    let results: Vec<serde_json::Value> = report.results.iter().map(|result| {
        serde_json::json!({
            "focusNode": result.focus_node.as_ref().map(|iri| iri.to_string()).unwrap_or_default(),
            "resultPath": result.result_path.as_ref().map(|p| p.to_string()).unwrap_or_default(),
            "value": result.value.as_ref().map(|v| v.to_string()).unwrap_or_default(),
            "sourceShape": result.source_shape.as_ref().map(|s| s.to_string()).unwrap_or_default(),
            "sourceConstraintComponent": result.source_constraint_component.to_string(),
            "message": result.message.clone(),
            "severity": match result.severity {
                fukurow_shacl::report::ViolationLevel::Info => "info",
                fukurow_shacl::report::ViolationLevel::Warning => "warning",
                fukurow_shacl::report::ViolationLevel::Violation => "violation",
            }
        })
    }).collect();

    let json_report = serde_json::json!({
        "conforms": report.conforms,
        "results": results
    });

    serde_json::to_string(&json_report)
        .map_err(|e| JsValue::from_str(&format!("JSON serialization error: {}", e)))
}

#[wasm_bindgen]
pub fn query_sparql(data_jsonld: &str, sparql: &str) -> Result<String, JsValue> {
    // Parse JSON-LD to RdfStore
    let store = jsonld_to_store(data_jsonld)?;

    // Execute SPARQL query
    let result = fukurow_sparql::execute_query(sparql, &store)
        .map_err(|e| JsValue::from_str(&format!("SPARQL execution error: {:?}", e)))?;

    // Convert result to JSON
    let json_result = match result {
        SparqlResult::Select { variables, bindings } => {
            serde_json::json!({
                "head": {
                    "vars": variables.iter().map(|v| v.0.clone()).collect::<Vec<_>>()
                },
                "results": {
                    "bindings": bindings.iter().map(|binding| {
                        let mut map = serde_json::Map::new();
                        for (var, term) in binding {
                    let value = match term {
                        fukurow_sparql::parser::Term::Iri(iri) => {
                            serde_json::json!({
                                "type": "uri",
                                "value": iri.0
                            })
                        },
                        fukurow_sparql::parser::Term::Literal(lit) => {
                            serde_json::json!({
                                "type": "literal",
                                "value": lit.value.trim_matches('"')
                            })
                        },
                        fukurow_sparql::parser::Term::Variable(var) => {
                            serde_json::json!({
                                "type": "variable",
                                "value": var.0
                            })
                        },
                        fukurow_sparql::parser::Term::BlankNode(bnode) => {
                            serde_json::json!({
                                "type": "bnode",
                                "value": bnode
                            })
                        },
                        fukurow_sparql::parser::Term::PrefixedName(prefix, local) => {
                            serde_json::json!({
                                "type": "prefixed",
                                "value": format!("{}:{}", prefix, local)
                            })
                        },
                    };
                            map.insert(var.0.clone(), value);
                        }
                        map
                    }).collect::<Vec<_>>()
                }
            })
        },
        SparqlResult::Ask { result } => {
            serde_json::json!({
                "head": {},
                "boolean": result
            })
        },
        SparqlResult::Construct { triples } => {
            // Convert constructed triples back to JSON-LD format
            let mut graph = Vec::new();
            for triple in triples {
                let mut node = serde_json::Map::new();
                node.insert("@id".to_string(), serde_json::Value::String(triple.subject));
                let mut properties = serde_json::Map::new();
                properties.insert(triple.predicate, serde_json::Value::String(triple.object));
                node.insert("properties".to_string(), serde_json::Value::Object(properties));
                graph.push(serde_json::Value::Object(node));
            }
            serde_json::json!({
                "@graph": graph
            })
        },
        SparqlResult::Describe { triples } => {
            // Similar to Construct
            let mut graph = Vec::new();
            for triple in triples {
                let mut node = serde_json::Map::new();
                node.insert("@id".to_string(), serde_json::Value::String(triple.subject));
                let mut properties = serde_json::Map::new();
                properties.insert(triple.predicate, serde_json::Value::String(triple.object));
                node.insert("properties".to_string(), serde_json::Value::Object(properties));
                graph.push(serde_json::Value::Object(node));
            }
            serde_json::json!({
                "@graph": graph
            })
        },
    };

    serde_json::to_string(&json_result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialization error: {}", e)))
}
