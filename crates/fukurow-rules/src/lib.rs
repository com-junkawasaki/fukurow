//! # Fukurow Rules
//!
//! Rule traits and constraint validation (SHACL equivalent)
//! Domain and policy rules for knowledge validation
//! Declarative security policy DSL for rule definition

pub mod traits;
pub mod dsl;

pub use traits::*;
pub use dsl::*;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use fukurow_core::model::{Triple, SecurityAction};
    use fukurow_store::store::RdfStore;
    use std::collections::HashMap;

    // Mock implementations for testing
    struct MockRule {
        name: &'static str,
        description: &'static str,
        priority: i32,
        should_apply_result: bool,
        rule_result: RuleResult,
    }

    impl MockRule {
        fn new(name: &'static str, description: &'static str, priority: i32) -> Self {
            Self {
                name,
                description,
                priority,
                should_apply_result: true,
                rule_result: RuleResult {
                    triples_to_add: vec![
                        Triple {
                            subject: "test_subject".to_string(),
                            predicate: "test_predicate".to_string(),
                            object: "test_object".to_string(),
                        }
                    ],
                    triples_to_remove: vec![],
                    actions: vec![
                        SecurityAction::Alert {
                            severity: "info".to_string(),
                            message: "Mock alert".to_string(),
                            details: serde_json::json!({"mock": true}),
                        }
                    ],
                    violations: vec![],
                    metadata: HashMap::new(),
                },
            }
        }

        fn with_should_apply(mut self, should_apply: bool) -> Self {
            self.should_apply_result = should_apply;
            self
        }
    }

    #[async_trait]
    impl Rule for MockRule {
        fn name(&self) -> &'static str {
            self.name
        }

        fn description(&self) -> &'static str {
            self.description
        }

        fn priority(&self) -> i32 {
            self.priority
        }

        async fn apply(&self, _store: &RdfStore) -> Result<RuleResult, RuleError> {
            Ok(self.rule_result.clone())
        }

        fn should_apply(&self, _store: &RdfStore) -> bool {
            self.should_apply_result
        }
    }

    struct MockValidationRule {
        name: &'static str,
        description: &'static str,
        violations: Vec<ValidationViolation>,
    }

    impl MockValidationRule {
        fn new(name: &'static str, description: &'static str) -> Self {
            Self {
                name,
                description,
                violations: vec![
                    ValidationViolation {
                        level: ViolationLevel::Warning,
                        message: "Mock violation".to_string(),
                        triple: Some(Triple {
                            subject: "test_subject".to_string(),
                            predicate: "test_predicate".to_string(),
                            object: "test_object".to_string(),
                        }),
                        rule_name: name.to_string(),
                        context: HashMap::new(),
                    }
                ],
            }
        }
    }

    #[async_trait]
    impl ValidationRule for MockValidationRule {
        fn name(&self) -> &'static str {
            self.name
        }

        fn description(&self) -> &'static str {
            self.description
        }

        async fn validate(&self, _store: &RdfStore) -> Result<Vec<ValidationViolation>, RuleError> {
            Ok(self.violations.clone())
        }
    }

    struct MockInferenceRule {
        name: &'static str,
        conditions: Vec<TriplePattern>,
        conclusions: Vec<TripleTemplate>,
    }

    impl MockInferenceRule {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                conditions: vec![
                    TriplePattern {
                        subject: PatternValue::Variable("s".to_string()),
                        predicate: PatternValue::Constant("type".to_string()),
                        object: PatternValue::Constant("Person".to_string()),
                    }
                ],
                conclusions: vec![
                    TripleTemplate {
                        subject: TemplateValue::Variable("s".to_string()),
                        predicate: TemplateValue::Constant("hasName".to_string()),
                        object: TemplateValue::Constant("Unknown".to_string()),
                    }
                ],
            }
        }
    }

    #[async_trait]
    impl InferenceRule for MockInferenceRule {
        fn name(&self) -> &'static str {
            self.name
        }

        fn conditions(&self) -> &[TriplePattern] {
            &self.conditions
        }

        fn conclusions(&self) -> &[TripleTemplate] {
            &self.conclusions
        }

        async fn infer(&self, bindings: &BindingMap, _store: &RdfStore) -> Result<Vec<Triple>, RuleError> {
            let mut triples = Vec::new();
            for template in &self.conclusions {
                match (&template.subject, &template.predicate, &template.object) {
                    (TemplateValue::Variable(s_var), TemplateValue::Constant(p), TemplateValue::Constant(o)) => {
                        if let Some(subject) = bindings.get(s_var) {
                            triples.push(Triple {
                                subject: subject.clone(),
                                predicate: p.clone(),
                                object: o.clone(),
                            });
                        }
                    }
                    _ => {} // Skip unsupported patterns for test
                }
            }
            Ok(triples)
        }
    }

    #[cfg(test)]
    mod rule_result_tests {
        use super::*;

        #[test]
        fn test_rule_result_creation() {
            let triples_to_add = vec![
                Triple {
                    subject: "subject1".to_string(),
                    predicate: "predicate1".to_string(),
                    object: "object1".to_string(),
                }
            ];

            let actions = vec![
                SecurityAction::Alert {
                    severity: "high".to_string(),
                    message: "Test alert".to_string(),
                    details: serde_json::json!({"test": true}),
                }
            ];

            let mut metadata = HashMap::new();
            metadata.insert("execution_time".to_string(), serde_json::json!(150));

            let result = RuleResult {
                triples_to_add: triples_to_add.clone(),
                triples_to_remove: vec![],
                actions: actions.clone(),
                violations: vec![],
                metadata: metadata.clone(),
            };

            assert_eq!(result.triples_to_add.len(), 1);
            assert_eq!(result.triples_to_remove.len(), 0);
            assert_eq!(result.actions.len(), 1);
            assert_eq!(result.violations.len(), 0);
            assert_eq!(result.metadata.get("execution_time"), Some(&serde_json::json!(150)));
        }

        #[test]
        fn test_rule_result_serialization() {
            let result = RuleResult {
                triples_to_add: vec![],
                triples_to_remove: vec![],
                actions: vec![],
                violations: vec![],
                metadata: HashMap::new(),
            };

            let json = serde_json::to_string(&result).unwrap();
            let deserialized: RuleResult = serde_json::from_str(&json).unwrap();

            assert_eq!(deserialized.triples_to_add.len(), 0);
            assert_eq!(deserialized.actions.len(), 0);
        }
    }

    #[cfg(test)]
    mod validation_violation_tests {
        use super::*;

        #[test]
        fn test_validation_violation_creation() {
            let triple = Triple {
                subject: "subject".to_string(),
                predicate: "predicate".to_string(),
                object: "object".to_string(),
            };

            let mut context = HashMap::new();
            context.insert("severity_score".to_string(), serde_json::json!(0.8));

            let violation = ValidationViolation {
                level: ViolationLevel::Error,
                message: "Validation failed".to_string(),
                triple: Some(triple.clone()),
                rule_name: "test_rule".to_string(),
                context: context.clone(),
            };

            assert_eq!(violation.level, ViolationLevel::Error);
            assert_eq!(violation.message, "Validation failed");
            assert_eq!(violation.triple, Some(triple));
            assert_eq!(violation.rule_name, "test_rule");
            assert_eq!(violation.context.get("severity_score"), Some(&serde_json::json!(0.8)));
        }

        #[test]
        fn test_validation_violation_without_triple() {
            let violation = ValidationViolation {
                level: ViolationLevel::Info,
                message: "Info message".to_string(),
                triple: None,
                rule_name: "info_rule".to_string(),
                context: HashMap::new(),
            };

            assert_eq!(violation.level, ViolationLevel::Info);
            assert!(violation.triple.is_none());
        }

        #[test]
        fn test_violation_level_equality() {
            assert_eq!(ViolationLevel::Info, ViolationLevel::Info);
            assert_eq!(ViolationLevel::Warning, ViolationLevel::Warning);
            assert_eq!(ViolationLevel::Error, ViolationLevel::Error);
            assert_eq!(ViolationLevel::Critical, ViolationLevel::Critical);

            assert_ne!(ViolationLevel::Info, ViolationLevel::Warning);
        }
    }

    #[cfg(test)]
    mod pattern_template_tests {
        use super::*;

        #[test]
        fn test_triple_pattern_creation() {
            let pattern = TriplePattern {
                subject: PatternValue::Variable("s".to_string()),
                predicate: PatternValue::Constant("type".to_string()),
                object: PatternValue::Variable("o".to_string()),
            };

            match &pattern.subject {
                PatternValue::Variable(var) => assert_eq!(var, "s"),
                _ => panic!("Expected variable"),
            }

            match &pattern.predicate {
                PatternValue::Constant(const_val) => assert_eq!(const_val, "type"),
                _ => panic!("Expected constant"),
            }
        }

        #[test]
        fn test_triple_template_creation() {
            let template = TripleTemplate {
                subject: TemplateValue::Variable("person".to_string()),
                predicate: TemplateValue::Constant("name".to_string()),
                object: TemplateValue::Constant("John".to_string()),
            };

            match &template.subject {
                TemplateValue::Variable(var) => assert_eq!(var, "person"),
                _ => panic!("Expected variable"),
            }
        }

        #[test]
        fn test_pattern_value_equality() {
            assert_eq!(PatternValue::Variable("x".to_string()), PatternValue::Variable("x".to_string()));
            assert_eq!(PatternValue::Constant("type".to_string()), PatternValue::Constant("type".to_string()));
            assert_ne!(PatternValue::Variable("x".to_string()), PatternValue::Constant("x".to_string()));
        }

        #[test]
        fn test_template_value_equality() {
            assert_eq!(TemplateValue::Variable("y".to_string()), TemplateValue::Variable("y".to_string()));
            assert_eq!(TemplateValue::Constant("value".to_string()), TemplateValue::Constant("value".to_string()));
            assert_ne!(TemplateValue::Variable("y".to_string()), TemplateValue::Constant("y".to_string()));
        }
    }

    #[cfg(test)]
    mod rule_error_tests {
        use super::*;

        #[test]
        fn test_rule_error_variants() {
            let err1 = RuleError::ExecutionError { message: "exec failed".to_string() };
            assert!(err1.to_string().contains("exec failed"));

            let err2 = RuleError::ConfigurationError { message: "config error".to_string() };
            assert!(err2.to_string().contains("config error"));

            let err3 = RuleError::PatternMatchError { message: "match failed".to_string() };
            assert!(err3.to_string().contains("match failed"));

            let err4 = RuleError::TemplateError { message: "template error".to_string() };
            assert!(err4.to_string().contains("template error"));

            let err5 = RuleError::ValidationError { message: "validation failed".to_string() };
            assert!(err5.to_string().contains("validation failed"));

            let store_err = RuleError::StoreError(anyhow::anyhow!("store error"));
            assert!(store_err.to_string().contains("store error"));
        }
    }

    #[cfg(test)]
    mod rule_trait_tests {
        use super::*;

        #[tokio::test]
        async fn test_mock_rule_implementation() {
            let rule = MockRule::new("test_rule", "Test rule description", 10);
            let store = RdfStore::new();

            assert_eq!(rule.name(), "test_rule");
            assert_eq!(rule.description(), "Test rule description");
            assert_eq!(rule.priority(), 10);
            assert!(rule.should_apply(&store));

            let result = rule.apply(&store).await.unwrap();
            assert_eq!(result.triples_to_add.len(), 1);
            assert_eq!(result.actions.len(), 1);
        }

        #[tokio::test]
        async fn test_mock_rule_with_should_apply_false() {
            let rule = MockRule::new("conditional_rule", "Conditional rule", 5)
                .with_should_apply(false);
            let store = RdfStore::new();

            assert!(!rule.should_apply(&store));
        }

        #[tokio::test]
        async fn test_mock_validation_rule() {
            let rule = MockValidationRule::new("validation_rule", "Validation rule");
            let store = RdfStore::new();

            assert_eq!(rule.name(), "validation_rule");
            assert_eq!(rule.description(), "Validation rule");

            let violations = rule.validate(&store).await.unwrap();
            assert_eq!(violations.len(), 1);
            assert_eq!(violations[0].level, ViolationLevel::Warning);
            assert_eq!(violations[0].rule_name, "validation_rule");
        }

        #[tokio::test]
        async fn test_mock_inference_rule() {
            let rule = MockInferenceRule::new("inference_rule");
            let mut bindings = HashMap::new();
            bindings.insert("s".to_string(), "person123".to_string());
            let store = RdfStore::new();

            assert_eq!(rule.name(), "inference_rule");
            assert_eq!(rule.conditions().len(), 1);
            assert_eq!(rule.conclusions().len(), 1);

            let triples = rule.infer(&bindings, &store).await.unwrap();
            assert_eq!(triples.len(), 1);
            assert_eq!(triples[0].subject, "person123");
            assert_eq!(triples[0].predicate, "hasName");
            assert_eq!(triples[0].object, "Unknown");
        }
    }

    #[cfg(test)]
    mod rule_registry_tests {
        use super::*;

        #[test]
        fn test_rule_registry_creation() {
            let registry = RuleRegistry::new();
            assert_eq!(registry.rule_count(), 0);
            assert_eq!(registry.validation_rule_count(), 0);
            assert_eq!(registry.inference_rule_count(), 0);
        }

        #[test]
        fn test_rule_registry_default() {
            let registry = RuleRegistry::default();
            assert_eq!(registry.rule_count(), 0);
        }

        #[tokio::test]
        async fn test_register_and_apply_rules() {
            let mut registry = RuleRegistry::new();
            let rule = MockRule::new("test_rule", "Test rule", 0);
            registry.register_rule(Box::new(rule));

            assert_eq!(registry.rule_count(), 1);

            let store = RdfStore::new();
            let results = registry.apply_all_rules(&store).await.unwrap();

            assert_eq!(results.len(), 1);
            assert_eq!(results[0].triples_to_add.len(), 1);
        }

        #[tokio::test]
        async fn test_register_and_validate_rules() {
            let mut registry = RuleRegistry::new();
            let validation_rule = MockValidationRule::new("validation", "Validation rule");
            registry.register_validation_rule(Box::new(validation_rule));

            assert_eq!(registry.validation_rule_count(), 1);

            let store = RdfStore::new();
            let violations = registry.validate_all(&store).await.unwrap();

            assert_eq!(violations.len(), 1);
            assert_eq!(violations[0].level, ViolationLevel::Warning);
        }

        #[test]
        fn test_register_inference_rule() {
            let mut registry = RuleRegistry::new();
            let inference_rule = MockInferenceRule::new("inference");
            registry.register_inference_rule(Box::new(inference_rule));

            assert_eq!(registry.inference_rule_count(), 1);
        }

        #[tokio::test]
        async fn test_multiple_rules_execution() {
            let mut registry = RuleRegistry::new();

            let rule1 = MockRule::new("rule1", "First rule", 10);
            let rule2 = MockRule::new("rule2", "Second rule", 5);

            registry.register_rule(Box::new(rule1));
            registry.register_rule(Box::new(rule2));

            let store = RdfStore::new();
            let results = registry.apply_all_rules(&store).await.unwrap();

            assert_eq!(results.len(), 2);
            // Both rules should have been applied
            assert_eq!(results.iter().map(|r| r.triples_to_add.len()).sum::<usize>(), 2);
        }

        #[tokio::test]
        async fn test_empty_registry() {
            let registry = RuleRegistry::new();
            let store = RdfStore::new();

            let results = registry.apply_all_rules(&store).await.unwrap();
            assert_eq!(results.len(), 0);

            let violations = registry.validate_all(&store).await.unwrap();
            assert_eq!(violations.len(), 0);
        }
    }

    #[cfg(test)]
    mod binding_map_tests {
        use super::*;

        #[test]
        fn test_binding_map_usage() {
            let mut bindings: BindingMap = HashMap::new();
            bindings.insert("person".to_string(), "john_doe".to_string());
            bindings.insert("age".to_string(), "30".to_string());

            assert_eq!(bindings.get("person"), Some(&"john_doe".to_string()));
            assert_eq!(bindings.get("age"), Some(&"30".to_string()));
            assert_eq!(bindings.get("nonexistent"), None);
        }
    }
}
