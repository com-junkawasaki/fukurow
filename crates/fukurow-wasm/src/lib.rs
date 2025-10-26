//! WebAssembly bindings for Fukurow reasoning engine
//!
//! このクレートはブラウザ環境でFukurowの推論機能を
//! 利用するためのWebAssemblyバインディングを提供します。

mod utils;

use fukurow_core::model::Triple;
use fukurow_store::store::RdfStore;
use fukurow_lite::{OwlLiteReasoner, Ontology as OwlLiteOntology};
use fukurow_dl::{OwlDlReasoner, OwlDlOntology};
use wasm_bindgen::prelude::*;
use web_sys::{console, CanvasRenderingContext2d, HtmlCanvasElement};

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// WebAssembly bindings for Fukurow reasoning engine
#[wasm_bindgen]
pub struct FukurowEngine {
    lite_reasoner: OwlLiteReasoner,
    dl_reasoner: OwlDlReasoner,
    store: RdfStore,
}

#[wasm_bindgen]
impl FukurowEngine {
    /// Create a new Fukurow engine instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> FukurowEngine {
        // Initialize console logging
        utils::set_panic_hook();

        FukurowEngine {
            lite_reasoner: OwlLiteReasoner::new(),
            dl_reasoner: OwlDlReasoner::new(),
            store: RdfStore::new(),
        }
    }

    /// Load RDF data in Turtle format
    #[wasm_bindgen]
    pub fn load_turtle(&mut self, turtle_data: &str) -> Result<(), JsValue> {
        // Parse Turtle format and add triples to store
        // This is a simplified implementation
        log(&format!("Loading Turtle data: {} bytes", turtle_data.len()));

        // For now, just acknowledge the data
        // TODO: Implement proper Turtle parsing
        Ok(())
    }

    /// Load OWL ontology from JSON-LD
    #[wasm_bindgen]
    pub fn load_jsonld(&mut self, jsonld_data: &str) -> Result<(), JsValue> {
        log(&format!("Loading JSON-LD data: {} bytes", jsonld_data.len()));

        // Parse JSON-LD and convert to RDF triples
        // TODO: Implement JSON-LD to RDF conversion
        Ok(())
    }

    /// Add a single RDF triple
    #[wasm_bindgen]
    pub fn add_triple(&mut self, subject: &str, predicate: &str, object: &str) -> Result<(), JsValue> {
        let triple = Triple {
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
        };

        self.store.insert(triple, fukurow_store::provenance::GraphId::Named("wasm".to_string()), fukurow_store::provenance::Provenance::Sensor {
            source: "wasm".to_string(),
            confidence: Some(1.0),
        });

        log(&format!("Added triple: {} {} {}", subject, predicate, object));
        Ok(())
    }

    /// Check if the current knowledge base is consistent (OWL Lite)
    #[wasm_bindgen]
    pub fn check_consistency_lite(&mut self) -> Result<bool, JsValue> {
        log("Checking consistency with OWL Lite...");

        match self.lite_reasoner.load_ontology(&self.store) {
            Ok(ontology) => {
                match self.lite_reasoner.is_consistent(&ontology) {
                    Ok(is_consistent) => {
                        log(&format!("OWL Lite consistency check: {}", is_consistent));
                        Ok(is_consistent)
                    }
                    Err(e) => {
                        log(&format!("OWL Lite consistency check failed: {:?}", e));
                        Err(JsValue::from_str(&format!("Consistency check failed: {:?}", e)))
                    }
                }
            }
            Err(e) => {
                log(&format!("Failed to load ontology: {:?}", e));
                Err(JsValue::from_str(&format!("Failed to load ontology: {:?}", e)))
            }
        }
    }

    /// Check if the current knowledge base is consistent (OWL DL)
    #[wasm_bindgen]
    pub fn check_consistency_dl(&mut self) -> Result<bool, JsValue> {
        log("Checking consistency with OWL DL...");

        match self.dl_reasoner.load_ontology(&self.store) {
            Ok(ontology) => {
                match self.dl_reasoner.is_consistent(&ontology) {
                    Ok(is_consistent) => {
                        log(&format!("OWL DL consistency check: {}", is_consistent));
                        Ok(is_consistent)
                    }
                    Err(e) => {
                        log(&format!("OWL DL consistency check failed: {:?}", e));
                        Err(JsValue::from_str(&format!("Consistency check failed: {:?}", e)))
                    }
                }
            }
            Err(e) => {
                log(&format!("Failed to load ontology: {:?}", e));
                Err(JsValue::from_str(&format!("Failed to load ontology: {:?}", e)))
            }
        }
    }

    /// Get the number of triples in the knowledge base
    #[wasm_bindgen]
    pub fn get_triple_count(&self) -> usize {
        self.store.all_triples().values().map(|v| v.len()).sum()
    }

    /// Clear all data from the knowledge base
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.store.clear_all();
        log("Knowledge base cleared");
    }

    /// Render knowledge graph to HTML5 Canvas
    #[wasm_bindgen]
    pub fn render_graph(&self, canvas_id: &str) -> Result<(), JsValue> {
        log(&format!("Rendering graph to canvas: {}", canvas_id));

        // Get canvas context
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?
            .dyn_into::<HtmlCanvasElement>()?;

        let context = canvas
            .get_context("2d")?
            .ok_or("Failed to get 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;

        // Simple graph rendering
        self.render_simple_graph(&context)?;

        Ok(())
    }
}

impl FukurowEngine {
    /// Render a simple graph visualization
    fn render_simple_graph(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        // Clear canvas
        context.clear_rect(0.0, 0.0, 800.0, 600.0);

        // Set drawing style
        context.set_fill_style(&JsValue::from_str("#f0f0f0"));
        context.fill_rect(0.0, 0.0, 800.0, 600.0);

        // Draw title
        context.set_font("24px Arial");
        context.set_fill_style(&JsValue::from_str("#333"));
        context.fill_text("Fukurow Knowledge Graph", 20.0, 40.0)?;

        // Draw triple count
        let triple_count = self.get_triple_count();
        context.set_font("16px Arial");
        context.fill_text(&format!("Triples: {}", triple_count), 20.0, 70.0)?;

        // Simple node visualization
        if triple_count > 0 {
            let center_x = 400.0;
            let center_y = 300.0;
            let radius = 100.0;

            // Draw central node
            context.begin_path();
            context.arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI)?;
            context.set_fill_style(&JsValue::from_str("#4CAF50"));
            context.fill();

            // Draw node label
            context.set_font("14px Arial");
            context.set_fill_style(&JsValue::from_str("#fff"));
            context.fill_text("Ontology", center_x - 30.0, center_y + 5.0)?;
        }

        Ok(())
    }
}

/// Utility function to log to browser console
#[wasm_bindgen]
pub fn log(message: &str) {
    console::log_1(&JsValue::from_str(message));
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn main() {
    log("Fukurow WebAssembly module initialized");
}
