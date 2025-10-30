//! OWL DL 推論エンジン
//!
//! このクレートは OWL DL の完全実装を提供します:
//! - 拡張テーブルロー推論アルゴリズム (∃-rule, ∀-rule)
//! - 個体レベルの推論 (individual reasoning)
//! - 複雑なクラスコンストラクタ (intersectionOf, unionOf, etc.)
//! - 計算量分析と最適化

pub mod model;
pub mod tableau;
pub mod reasoner;
pub mod loader;

pub use model::{ClassExpression, OwlDlOntology};
pub use reasoner::OwlDlReasoner;
pub use loader::OwlDlOntologyLoader;

// Re-export OWL Lite types for compatibility
pub use fukurow_lite::{Ontology as OwlLiteOntology, Class, Property, Individual, Axiom as OwlLiteAxiom, model::OwlIri};

// Error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OwlDlError {
    #[error("Loader error: {0}")]
    LoaderError(String),

    #[error("Reasoning error: {0}")]
    ReasoningError(String),

    #[error("Consistency error: {0}")]
    ConsistencyError(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}
