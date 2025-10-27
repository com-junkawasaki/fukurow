//! SHACL 検証レポート

use crate::loader::ShapesGraph;
use fukurow_sparql::parser::{Iri, Literal, Term};
use serde::{Serialize, Deserialize};

/// Validation Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub conforms: bool,
    #[serde(skip)]
    pub results: Vec<ValidationResult>,
    #[serde(skip)]
    pub shapes_graph: Option<ShapesGraph>,
}

/// Validation Result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub focus_node: Option<Iri>,

    pub result_path: Option<Iri>,

    pub value: Option<String>,

    pub source_constraint_component: Iri,

    pub source_shape: Option<Iri>,

    pub detail: Option<Box<ValidationResult>>,

    pub message: Option<String>,

    pub severity: ViolationLevel,
}

/// Violation Level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationLevel {
    Violation,

    Warning,

    Info,
}

impl ValidationReport {
    /// JSON-LD 形式でシリアライズ
    pub fn to_jsonld(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Turtle 形式でシリアライズ
    pub fn to_turtle(&self) -> String {
        // TODO: Turtle シリアライズ実装
        format!("@prefix sh: <http://www.w3.org/ns/shacl#> .\n\n{}", self.to_simple_string())
    }

    /// 人間可読形式で出力
    pub fn to_simple_string(&self) -> String {
        let mut output = format!("Validation Report: {}\n", if self.conforms { "CONFORMS" } else { "DOES NOT CONFORM" });

        for (i, result) in self.results.iter().enumerate() {
            output.push_str(&format!("Violation {}: {}\n", i + 1,
                result.message.as_ref().unwrap_or(&"No message".to_string())));

            if let Some(node) = &result.focus_node {
                output.push_str(&format!("  Focus Node: {}\n", node));
            }

            if let Some(value) = &result.value {
                output.push_str(&format!("  Value: {:?}\n", value));
            }

            output.push_str(&format!("  Severity: {:?}\n", result.severity));
            output.push_str(&format!("  Constraint: {}\n", result.source_constraint_component));
            output.push('\n');
        }

        output
    }

    /// 違反の数を取得
    pub fn violation_count(&self) -> usize {
        self.results.iter().filter(|r| matches!(r.severity, ViolationLevel::Violation)).count()
    }

    /// 警告の数を取得
    pub fn warning_count(&self) -> usize {
        self.results.iter().filter(|r| matches!(r.severity, ViolationLevel::Warning)).count()
    }

    /// 情報メッセージの数を取得
    pub fn info_count(&self) -> usize {
        self.results.iter().filter(|r| matches!(r.severity, ViolationLevel::Info)).count()
    }
}

impl ValidationResult {
    /// 違反結果を作成
    pub fn violation(
        focus_node: Option<Iri>,
        constraint: Iri,
        message: String,
    ) -> Self {
        Self {
            focus_node,
            result_path: None,
            value: None,
            source_constraint_component: constraint,
            source_shape: None,
            detail: None,
            message: Some(message),
            severity: ViolationLevel::Violation,
        }
    }

    /// 警告結果を作成
    pub fn warning(
        focus_node: Option<Iri>,
        constraint: Iri,
        message: String,
    ) -> Self {
        Self {
            focus_node,
            result_path: None,
            value: None,
            source_constraint_component: constraint,
            source_shape: None,
            detail: None,
            message: Some(message),
            severity: ViolationLevel::Warning,
        }
    }

    /// 情報結果を作成
    pub fn info(
        focus_node: Option<Iri>,
        constraint: Iri,
        message: String,
    ) -> Self {
        Self {
            focus_node,
            result_path: None,
            value: None,
            source_constraint_component: constraint,
            source_shape: None,
            detail: None,
            message: Some(message),
            severity: ViolationLevel::Info,
        }
    }
}
