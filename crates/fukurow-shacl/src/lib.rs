//! SHACL Core + SHACL-SPARQL 検証エンジン
//!
//! このクレートは W3C SHACL 仕様の完全実装を提供します:
//! - ShapesGraph 読み込み (Loader)
//! - 制約検証 (Validator)
//! - 検証レポート (Report)

pub mod loader;
pub mod validator;
pub mod report;

// Re-exports
pub use loader::{ShaclLoader, ShapesGraph, Shape, PropertyShape, NodeShape};
pub use validator::{ShaclValidator, ValidationConfig, ValidationMode};
pub use report::{ValidationReport, ValidationResult, ViolationLevel};

// Error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShaclError {
    #[error("Loader error: {0}")]
    LoaderError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Report error: {0}")]
    ReportError(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}
