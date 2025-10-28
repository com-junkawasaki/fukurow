//! WebAssembly bindings for Fukurow reasoning engine
//!
//! このクレートはブラウザ環境でFukurowの推論機能を
//! 利用するためのWebAssemblyバインディングを提供します。

mod utils;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{console, CanvasRenderingContext2d, HtmlCanvasElement};

/// RDF Triple representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Triple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}
use std::collections::HashSet;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// Simplified WebAssembly bindings for Fukurow reasoning engine
/// This version uses only fukurow-core for WebAssembly compatibility
#[wasm_bindgen]
pub struct FukurowEngine {
    triples: Vec<Triple>,
}

#[wasm_bindgen]
impl FukurowEngine {
    /// Create a new Fukurow engine instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> FukurowEngine {
        // Initialize console logging
        utils::set_panic_hook();

        FukurowEngine {
            triples: Vec::new(),
        }
    }

    /// Add a single RDF triple
    #[wasm_bindgen]
    pub fn add_triple(&mut self, subject: &str, predicate: &str, object: &str) -> Result<(), JsValue> {
        let triple = Triple {
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
        };

        self.triples.push(triple);
        log(&format!("Added triple: {} {} {}", subject, predicate, object));
        Ok(())
    }

    /// Get the number of triples in the knowledge base
    #[wasm_bindgen]
    pub fn get_triple_count(&self) -> usize {
        self.triples.len()
    }

    /// Clear all data from the knowledge base
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.triples.clear();
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

        // Simple visualization based on triple count
        if triple_count > 0 {
            let center_x = 400.0;
            let center_y = 300.0;
            let radius = (triple_count as f64).min(100.0).max(20.0);

            // Draw central node
            context.begin_path();
            context.arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI)?;
            context.set_fill_style(&JsValue::from_str("#4CAF50"));
            context.fill();

            // Draw node label
            context.set_font("14px Arial");
            context.set_fill_style(&JsValue::from_str("#fff"));
            context.fill_text("Knowledge Base", center_x - 50.0, center_y + 5.0)?;
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
