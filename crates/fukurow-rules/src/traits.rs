//! Rule traits and interfaces

use async_trait::async_trait;
use fukurow_core::model::{Triple, SecurityAction};
use fukurow_store::store::RdfStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of rule application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    /// New triples to add
    pub triples_to_add: Vec<Triple>,
    /// Triples to remove
    pub triples_to_remove: Vec<Triple>,
    /// Security actions to execute
    pub actions: Vec<SecurityAction>,
    /// Validation violations
    pub violations: Vec<ValidationViolation>,
    /// Rule execution metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Validation violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationViolation {
    /// Violation level
    pub level: ViolationLevel,
    /// Human-readable message
    pub message: String,
    /// Violating triple (if applicable)
    pub triple: Option<Triple>,
    /// Rule that detected the violation
    pub rule_name: String,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Violation severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViolationLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Core rule trait
#[async_trait]
pub trait Rule: Send + Sync {
    /// Get the rule name
    fn name(&self) -> &'static str;

    /// Get the rule description
    fn description(&self) -> &'static str;

    /// Get the rule priority (higher = executed first)
    fn priority(&self) -> i32 { 0 }

    /// Apply the rule to a graph
    async fn apply(&self, store: &RdfStore) -> Result<RuleResult, RuleError>;

    /// Check if this rule should be applied to the given graph
    fn should_apply(&self, store: &RdfStore) -> bool { true }
}

/// Validation rule trait (subset of Rule)
#[async_trait]
pub trait ValidationRule: Send + Sync {
    /// Get the rule name
    fn name(&self) -> &'static str;

    /// Get the rule description
    fn description(&self) -> &'static str;

    /// Validate a graph and return violations
    async fn validate(&self, store: &RdfStore) -> Result<Vec<ValidationViolation>, RuleError>;
}

/// Inference rule trait
#[async_trait]
pub trait InferenceRule: Send + Sync {
    /// Get the rule name
    fn name(&self) -> &'static str;

    /// Get inference conditions (antecedent)
    fn conditions(&self) -> &[TriplePattern];

    /// Get inference conclusions (consequent)
    fn conclusions(&self) -> &[TripleTemplate];

    /// Apply inference to matching patterns
    async fn infer(&self, bindings: &BindingMap, store: &RdfStore) -> Result<Vec<Triple>, RuleError>;
}

/// Triple pattern for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriplePattern {
    pub subject: PatternValue,
    pub predicate: PatternValue,
    pub object: PatternValue,
}

/// Triple template for generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripleTemplate {
    pub subject: TemplateValue,
    pub predicate: TemplateValue,
    pub object: TemplateValue,
}

/// Pattern value (variable or constant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternValue {
    Variable(String),
    Constant(String),
}

/// Template value (variable reference or constant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateValue {
    Variable(String),
    Constant(String),
}

/// Variable bindings map
pub type BindingMap = HashMap<String, String>;

/// Rule execution errors
#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("Rule execution failed: {message}")]
    ExecutionError { message: String },

    #[error("Invalid rule configuration: {message}")]
    ConfigurationError { message: String },

    #[error("Pattern matching failed: {message}")]
    PatternMatchError { message: String },

    #[error("Template instantiation failed: {message}")]
    TemplateError { message: String },

    #[error("Validation failed: {message}")]
    ValidationError { message: String },

    #[error("Storage operation failed: {0}")]
    StoreError(#[from] anyhow::Error),
}

/// Rule registry for managing multiple rules
pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
    validation_rules: Vec<Box<dyn ValidationRule>>,
    inference_rules: Vec<Box<dyn InferenceRule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            validation_rules: Vec::new(),
            inference_rules: Vec::new(),
        }
    }

    /// Register a general rule
    pub fn register_rule(&mut self, rule: Box<dyn Rule>) {
        self.rules.push(rule);
    }

    /// Register a validation rule
    pub fn register_validation_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.validation_rules.push(rule);
    }

    /// Register an inference rule
    pub fn register_inference_rule(&mut self, rule: Box<dyn InferenceRule>) {
        self.inference_rules.push(rule);
    }

    /// Apply all rules to a store
    pub async fn apply_all_rules(&self, store: &RdfStore) -> Result<Vec<RuleResult>, RuleError> {
        let mut results = Vec::new();

        for rule in &self.rules {
            if rule.should_apply(store) {
                let result = rule.apply(store).await?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Run all validation rules
    pub async fn validate_all(&self, store: &RdfStore) -> Result<Vec<ValidationViolation>, RuleError> {
        let mut all_violations = Vec::new();

        for rule in &self.validation_rules {
            let violations = rule.validate(store).await?;
            all_violations.extend(violations);
        }

        Ok(all_violations)
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
