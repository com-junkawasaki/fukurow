//! Reasoning engine orchestration

use async_trait::async_trait;
use fukurow_core::model::{Triple, SecurityAction};
use fukurow_store::store::RdfStore;
use fukurow_rules::{Rule, RuleResult, RuleRegistry};
use fukurow_rdfs::{RdfsReasoner, RdfsConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Engine result containing all outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineResult {
    /// Inferred triples added
    pub inferred_triples: Vec<Triple>,
    /// Security actions to execute
    pub actions: Vec<SecurityAction>,
    /// Validation violations found
    pub violations: Vec<fukurow_rules::ValidationViolation>,
    /// Processing statistics
    pub stats: ProcessingStats,
}

/// Processing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub rules_applied: usize,
    pub triples_processed: usize,
    pub execution_time_ms: u64,
    pub memory_used_kb: Option<usize>,
}

/// Reasoning engine that orchestrates the entire process
pub struct ReasoningEngine {
    rule_registry: RuleRegistry,
    processing_options: ProcessingOptions,
}

#[derive(Debug, Clone)]
pub struct ProcessingOptions {
    pub max_iterations: usize,
    pub enable_validation: bool,
    pub enable_inference: bool,
    pub enable_rdfs_inference: bool,
    pub timeout_ms: Option<u64>,
    pub rdfs_config: RdfsConfig,
}

impl ReasoningEngine {
    pub fn new() -> Self {
        Self {
            rule_registry: RuleRegistry::new(),
            processing_options: ProcessingOptions::default(),
        }
    }

    pub fn with_options(options: ProcessingOptions) -> Self {
        Self {
            rule_registry: RuleRegistry::new(),
            processing_options: options,
        }
    }

    /// Register a rule
    pub fn register_rule(&mut self, rule: Box<dyn Rule>) {
        self.rule_registry.register_rule(rule);
    }

    /// Process a knowledge graph through all reasoning steps
    pub async fn process(&self, store: &RdfStore) -> Result<EngineResult, EngineError> {
        let start_time = std::time::Instant::now();

        let mut result = EngineResult {
            inferred_triples: Vec::new(),
            actions: Vec::new(),
            violations: Vec::new(),
            stats: ProcessingStats {
                rules_applied: 0,
                triples_processed: store.statistics().total_triples,
                execution_time_ms: 0,
                memory_used_kb: None,
            },
        };

        // RDFS inference (first step)
        if self.processing_options.enable_rdfs_inference {
            let mut rdfs_reasoner = RdfsReasoner::new();
            let rdfs_triples = rdfs_reasoner.compute_closure(store)?;
            result.inferred_triples.extend(rdfs_triples);
            result.stats.rules_applied += 1; // Count RDFS as one "rule"
        }

        // Apply all rules
        if self.processing_options.enable_inference {
            let rule_results = self.rule_registry.apply_all_rules(store).await?;

            for rule_result in rule_results {
                result.inferred_triples.extend(rule_result.triples_to_add);
                result.actions.extend(rule_result.actions);
                result.violations.extend(rule_result.violations);
                result.stats.rules_applied += 1;
            }
        }

        // Run validation if enabled
        if self.processing_options.enable_validation {
            let violations = self.rule_registry.validate_all(store).await?;
            result.violations.extend(violations);
        }

        result.stats.execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(result)
    }

    /// Get rule registry for inspection
    pub fn rule_registry(&self) -> &RuleRegistry {
        &self.rule_registry
    }

    /// Get processing options
    pub fn processing_options(&self) -> &ProcessingOptions {
        &self.processing_options
    }
}

/// Processing errors
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Rule execution failed: {0}")]
    RuleError(#[from] fukurow_rules::RuleError),

    #[error("RDFS reasoning failed: {0}")]
    RdfsError(#[from] fukurow_rdfs::RdfsError),

    #[error("Processing timeout after {0}ms")]
    TimeoutError(u64),

    #[error("Maximum iterations ({0}) exceeded")]
    IterationLimitError(usize),

    #[error("Internal engine error: {0}")]
    InternalError(String),
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            enable_validation: true,
            enable_inference: true,
            enable_rdfs_inference: true, // RDFS推論をデフォルトで有効化
            timeout_ms: Some(5000), // 5 seconds
            rdfs_config: RdfsConfig::default(),
        }
    }
}

impl Default for ReasoningEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline for chaining multiple reasoning engines
pub struct ReasoningPipeline {
    engines: Vec<ReasoningEngine>,
}

impl ReasoningPipeline {
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    pub fn add_engine(&mut self, engine: ReasoningEngine) {
        self.engines.push(engine);
    }

    /// Execute pipeline on a store
    pub async fn execute(&self, store: &RdfStore) -> Result<PipelineResult, EngineError> {
        let mut combined_result = PipelineResult {
            engine_results: Vec::new(),
            total_stats: ProcessingStats {
                rules_applied: 0,
                triples_processed: store.statistics().total_triples,
                execution_time_ms: 0,
                memory_used_kb: None,
            },
        };

        let start_time = std::time::Instant::now();

        for engine in &self.engines {
            let result = engine.process(store).await?;
            combined_result.engine_results.push(result);
        }

        // Aggregate statistics
        for engine_result in &combined_result.engine_results {
            combined_result.total_stats.rules_applied += engine_result.stats.rules_applied;
        }

        combined_result.total_stats.execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(combined_result)
    }
}

/// Pipeline execution result
#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub engine_results: Vec<EngineResult>,
    pub total_stats: ProcessingStats,
}

impl Default for ReasoningPipeline {
    fn default() -> Self {
        Self::new()
    }
}
