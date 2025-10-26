//! SPARQL 1.1 エンジン
//!
//! このクレートは SPARQL 1.1 の完全実装を提供します:
//! - 構文解析 (Parser)
//! - 論理代数変換 (Algebra)
//! - クエリ最適化 (Optimizer)
//! - 実行エンジン (Evaluator)

pub mod parser;
pub mod algebra;
pub mod optimizer;
pub mod evaluator;

// Re-exports
pub use parser::{SparqlParser, SparqlQuery, QueryType};
pub use algebra::{Algebra, PlanBuilder};
pub use optimizer::{SparqlOptimizer, OptimizationRule};
pub use evaluator::{SparqlEvaluator, QueryResult};
pub use parser::Bindings;

// Error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SparqlError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Algebra error: {0}")]
    AlgebraError(String),

    #[error("Optimization error: {0}")]
    OptimizationError(String),

    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}
