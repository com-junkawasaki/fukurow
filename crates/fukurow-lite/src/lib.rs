//! OWL Lite 推論エンジン
//!
//! このクレートは OWL Lite の完全実装を提供します:
//! - テーブルロー推論アルゴリズム
//! - 整合性検証
//! - クラス階層推論
//! - インスタンス検証

pub mod model;
pub mod tableau;
pub mod reasoner;
pub mod loader;

pub use model::{Ontology, Class, Property, Individual, Axiom};
pub use reasoner::OwlLiteReasoner;
pub use loader::OntologyLoader;

// Error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OwlError {
    #[error("Loader error: {0}")]
    LoaderError(String),

    #[error("Reasoning error: {0}")]
    ReasoningError(String),

    #[error("Consistency error: {0}")]
    ConsistencyError(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}
