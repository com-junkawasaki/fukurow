//! Headless WebAssembly bindings for Fukurow reasoning engine
//!
//! DOM/Node 依存を一切持たない、純計算 API を提供します。

use wasm_bindgen::prelude::*;
use serde::Deserialize;
use fukurow_store::store::RdfStore;
use fukurow_store::provenance::{Provenance, GraphId};
use fukurow_core::model::Triple;
use fukurow_core::jsonld::{parse_jsonld, serialize_jsonld, jsonld_to_triples};
use fukurow_lite::{OwlLiteReasoner, OntologyLoader};
use fukurow_lite::loader::DefaultOntologyLoader;
// Note: SPARQL and SHACL are disabled for WASM due to logos/mio dependencies
// use fukurow_sparql::{execute_query as sparql_execute, QueryResult as SparqlResult};
// use fukurow_shacl::{ShaclValidator, ShaclLoader, ValidationReport};
// use fukurow_shacl::loader::DefaultShaclLoader;
// use fukurow_shacl::validator::DefaultShaclValidator;

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

fn jsonld_to_store(jsonld_str: &str) -> Result<RdfStore, JsValue> {
    let mut store = RdfStore::new();
    let doc = parse_jsonld(jsonld_str)
        .map_err(|e| JsValue::from_str(&format!("JSON-LD parse error: {}", e)))?;

    let triples = jsonld_to_triples(&doc)
        .map_err(|e| JsValue::from_str(&format!("Triple conversion error: {}", e)))?;

    let graph_id = GraphId::Default;
    let provenance = Provenance::Sensor {
        source: "wasm-input".to_string(),
        confidence: Some(1.0),
    };

    for triple in triples {
        store.insert(triple, graph_id.clone(), provenance.clone());
    }

    Ok(store)
}

fn store_to_jsonld(store: &RdfStore) -> Result<String, JsValue> {
    // Convert store triples back to fukurow-core JsonLdDocument
    // This is a simplified implementation - in practice, you might want more sophisticated conversion
    let mut graph = Vec::new();

    for stored_triple in store.all_triples().values().flatten() {
        let triple = &stored_triple.triple;

        // Create a simple JSON-LD node
        let mut node = serde_json::Map::new();
        node.insert("@id".to_string(), serde_json::Value::String(triple.subject.clone()));

        // For simplicity, we'll create a flat structure
        // In a real implementation, you'd want to group by subject
        let mut properties = serde_json::Map::new();
        properties.insert(triple.predicate.clone(), serde_json::Value::String(triple.object.clone()));
        node.insert("properties".to_string(), serde_json::Value::Object(properties));

        graph.push(serde_json::Value::Object(node));
    }

    let doc = fukurow_core::model::JsonLdDocument {
        context: serde_json::json!({
            "@vocab": "http://www.w3.org/2002/07/owl#"
        }),
        graph: Some(graph),
        data: std::collections::HashMap::new(),
    };

    serialize_jsonld(&doc)
        .map_err(|e| JsValue::from_str(&format!("JSON-LD serialize error: {}", e)))
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
    // TODO: Implement SHACL validation when logos/mio WASM compatibility is resolved
    // For now, return basic conformant result
    let _data = data_jsonld;
    let _shapes = shape_jsonld;
    Ok(r#"{"conforms":true,"results":[]}"#.to_string())
}

#[wasm_bindgen]
pub fn query_sparql(data_jsonld: &str, sparql: &str) -> Result<String, JsValue> {
    // TODO: Implement SPARQL query when logos/mio WASM compatibility is resolved
    // For now, return empty results
    let _data = data_jsonld;
    let _sparql = sparql;
    Ok(r#"{"head":{"vars":[]},"results":{"bindings":[]}}"#.to_string())
}
