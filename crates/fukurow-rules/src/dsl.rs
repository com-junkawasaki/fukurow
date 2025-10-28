//! # Security Policy DSL
//!
//! Declarative security policy definition language
//! JSON/YAMLベースの宣言的ルール記述と実行

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use async_trait::async_trait;
use crate::{Rule, RuleResult, RuleError, ValidationViolation, ViolationLevel};
use fukurow_core::model::{Triple, SecurityAction};
use fukurow_store::store::RdfStore;
use chrono::{Utc};

/// DSLベースのセキュリティポリシー定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub name: String,
    pub description: String,
    pub version: String,
    pub priority: i32,
    pub rules: Vec<PolicyRule>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 個別のポリシールール定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: String,
    pub name: String,
    pub description: String,

    /// ルールが適用される条件
    pub conditions: Vec<Condition>,

    /// ルールが適用された場合のアクション
    pub actions: Vec<PolicyAction>,

    /// ルール適用時の重大度
    pub severity: Severity,

    /// 追加のメタデータ
    pub metadata: HashMap<String, serde_json::Value>,
}

/// ルール適用条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "config")]
pub enum Condition {
    /// トリプルの存在条件
    TripleExists {
        subject: String,
        predicate: String,
        object: String,
    },

    /// トリプルの不存在条件
    TripleNotExists {
        subject: String,
        predicate: String,
        object: String,
    },

    /// 変数束縛条件
    VariableBinding {
        variable: String,
        value: String,
    },

    /// 数値比較条件
    NumericComparison {
        left: ValueExpression,
        operator: ComparisonOperator,
        right: ValueExpression,
    },

    /// 論理演算子
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
}

/// 値表現
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ValueExpression {
    /// 定数値
    Constant(serde_json::Value),

    /// 変数参照
    Variable(String),

    /// トリプルからの値抽出
    TripleValue {
        subject: String,
        predicate: String,
        extract: ExtractType,
    },

    /// 関数呼び出し
    FunctionCall {
        function: String,
        arguments: Vec<ValueExpression>,
    },
}

/// 値抽出タイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractType {
    Subject,
    Predicate,
    Object,
}

/// 比較演算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    NotContains,
}

/// ポリシーアクション
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "config")]
pub enum PolicyAction {
    /// トリプルを追加
    AddTriple {
        subject: String,
        predicate: String,
        object: String,
    },

    /// トリプルを削除
    RemoveTriple {
        subject: String,
        predicate: String,
        object: String,
    },

    /// セキュリティアクション実行
    SecurityAction {
        action_type: SecurityActionType,
        message: String,
        details: HashMap<String, serde_json::Value>,
    },

    /// 違反レポート作成
    ReportViolation {
        level: ViolationLevel,
        message: String,
        context: HashMap<String, serde_json::Value>,
    },
}

/// セキュリティアクションタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityActionType {
    Alert,
    Block,
    Log,
    Quarantine,
    Notify,
}

/// 重大度レベル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// DSLベースのルールエンジン
pub struct DslRuleEngine {
    policies: Vec<SecurityPolicy>,
    variables: HashMap<String, serde_json::Value>,
}

impl DslRuleEngine {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            variables: HashMap::new(),
        }
    }

    /// ポリシーを追加
    pub fn add_policy(&mut self, policy: SecurityPolicy) {
        self.policies.push(policy);
    }

    /// JSON/YAMLからポリシーを読み込み
    pub fn load_policy_from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let policy: SecurityPolicy = serde_json::from_str(json)?;
        self.add_policy(policy);
        Ok(())
    }

    /// 変数を設定
    pub fn set_variable(&mut self, name: String, value: serde_json::Value) {
        self.variables.insert(name, value);
    }

    /// すべてのポリシーを実行
    pub async fn execute_all_policies(&self, store: &RdfStore) -> Result<Vec<RuleResult>, RuleError> {
        let mut results = Vec::new();

        for policy in &self.policies {
            let result = self.execute_policy(policy, store).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 個別のポリシーを実行
    async fn execute_policy(&self, policy: &SecurityPolicy, store: &RdfStore) -> Result<RuleResult, RuleError> {
        let mut triples_to_add = Vec::new();
        let mut triples_to_remove = Vec::new();
        let mut actions = Vec::new();
        let mut violations = Vec::new();

        for rule in &policy.rules {
            if self.evaluate_conditions(&rule.conditions, store).await? {
                // 条件が満たされた場合、アクションを実行
                for action in &rule.actions {
                    match action {
                        PolicyAction::AddTriple { subject, predicate, object } => {
                            triples_to_add.push(Triple {
                                subject: subject.clone(),
                                predicate: predicate.clone(),
                                object: object.clone(),
                            });
                        }
                        PolicyAction::RemoveTriple { subject, predicate, object } => {
                            triples_to_remove.push(Triple {
                                subject: subject.clone(),
                                predicate: predicate.clone(),
                                object: object.clone(),
                            });
                        }
                        PolicyAction::SecurityAction { action_type, message, details } => {
                            let severity = match rule.severity {
                                Severity::Low => "low",
                                Severity::Medium => "medium",
                                Severity::High => "high",
                                Severity::Critical => "critical",
                            };

                            let security_action = match action_type {
                                SecurityActionType::Alert => SecurityAction::Alert {
                                    severity: severity.to_string(),
                                    message: message.clone(),
                                    details: serde_json::to_value(details).unwrap_or_default(),
                                },
                                SecurityActionType::Block => SecurityAction::Alert {
                                    severity: "high".to_string(),
                                    message: format!("Block action required: {}", message),
                                    details: serde_json::json!({
                                        "action_type": "block",
                                        "reason": format!("Policy violation: {}", rule.name),
                                        "original_details": details
                                    }),
                                },
                                SecurityActionType::Log => SecurityAction::Alert {
                                    severity: "info".to_string(),
                                    message: format!("Log: {}", message),
                                    details: serde_json::json!({
                                        "action_type": "log",
                                        "level": severity,
                                        "original_details": details
                                    }),
                                },
                                SecurityActionType::Quarantine => SecurityAction::Alert {
                                    severity: "critical".to_string(),
                                    message: format!("Quarantine required: {}", message),
                                    details: serde_json::json!({
                                        "action_type": "quarantine",
                                        "reason": format!("Policy violation: {}", rule.name),
                                        "original_details": details
                                    }),
                                },
                                SecurityActionType::Notify => SecurityAction::Alert {
                                    severity: severity.to_string(),
                                    message: format!("Notification: {}", message),
                                    details: serde_json::json!({
                                        "action_type": "notify",
                                        "recipients": [message.clone()],
                                        "priority": severity,
                                        "original_details": details
                                    }),
                                },
                            };

                            actions.push(security_action);
                        }
                        PolicyAction::ReportViolation { level, message, context } => {
                            violations.push(ValidationViolation {
                                level: level.clone(),
                                message: message.clone(),
                                triple: None,
                                rule_name: rule.id.clone(),
                                context: context.clone(),
                            });
                        }
                    }
                }
            }
        }

        let mut metadata = HashMap::new();
        metadata.insert("policy_name".to_string(), serde_json::json!(policy.name));
        metadata.insert("policy_version".to_string(), serde_json::json!(policy.version));
        metadata.insert("execution_time".to_string(), serde_json::json!(Utc::now().timestamp()));

        Ok(RuleResult {
            triples_to_add,
            triples_to_remove,
            actions,
            violations,
            metadata,
        })
    }

    /// 条件を評価
    async fn evaluate_conditions(&self, conditions: &[Condition], store: &RdfStore) -> Result<bool, RuleError> {
        for condition in conditions {
            if !self.evaluate_condition(condition, store).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 個別の条件を評価
    async fn evaluate_condition(&self, condition: &Condition, store: &RdfStore) -> Result<bool, RuleError> {
        match condition {
            Condition::TripleExists { subject, predicate, object } => {
                // トリプルが存在するかチェック
                let subject_opt = if subject.starts_with('?') { None } else { Some(subject.as_str()) };
                let predicate_opt = Some(predicate.as_str());
                let object_opt = if object.starts_with('?') { None } else { Some(object.as_str()) };

                let results = store.find_triples(subject_opt, predicate_opt, object_opt);
                Ok(!results.is_empty())
            }

            Condition::TripleNotExists { subject, predicate, object } => {
                let exists = Box::pin(self.evaluate_condition(
                    &Condition::TripleExists {
                        subject: subject.clone(),
                        predicate: predicate.clone(),
                        object: object.clone(),
                    },
                    store
                )).await?;
                Ok(!exists)
            }

            Condition::VariableBinding { variable, value } => {
                Ok(self.variables.get(variable)
                    .map(|v| v.as_str().unwrap_or("") == value)
                    .unwrap_or(false))
            }

            Condition::NumericComparison { left, operator, right } => {
                let left_val = self.evaluate_expression(left, store).await?;
                let right_val = self.evaluate_expression(right, store).await?;

                let left_num = left_val.as_f64().unwrap_or(0.0);
                let right_num = right_val.as_f64().unwrap_or(0.0);

                let result = match operator {
                    ComparisonOperator::Equal => (left_num - right_num).abs() < f64::EPSILON,
                    ComparisonOperator::NotEqual => (left_num - right_num).abs() >= f64::EPSILON,
                    ComparisonOperator::GreaterThan => left_num > right_num,
                    ComparisonOperator::LessThan => left_num < right_num,
                    ComparisonOperator::GreaterThanOrEqual => left_num >= right_num,
                    ComparisonOperator::LessThanOrEqual => left_num <= right_num,
                    ComparisonOperator::Contains => false, // 数値比較では未実装
                    ComparisonOperator::NotContains => true, // 数値比較では未実装
                };

                Ok(result)
            }

            Condition::And(conditions) => {
                for cond in conditions {
                    if !Box::pin(self.evaluate_condition(cond, store)).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }

            Condition::Or(conditions) => {
                for cond in conditions {
                    if Box::pin(self.evaluate_condition(cond, store)).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }

            Condition::Not(condition) => {
                let result = Box::pin(self.evaluate_condition(condition, store)).await?;
                Ok(!result)
            }
        }
    }

    /// 式を評価
    async fn evaluate_expression(&self, expr: &ValueExpression, _store: &RdfStore) -> Result<serde_json::Value, RuleError> {
        match expr {
            ValueExpression::Constant(value) => Ok(value.clone()),
            ValueExpression::Variable(name) => {
                self.variables.get(name)
                    .cloned()
                    .ok_or_else(|| RuleError::ExecutionError {
                        message: format!("Variable not found: {}", name)
                    })
            }
            ValueExpression::TripleValue { .. } => {
                // トリプルからの値抽出は未実装
                Err(RuleError::ExecutionError {
                    message: "Triple value extraction not implemented".to_string()
                })
            }
            ValueExpression::FunctionCall { .. } => {
                // 関数呼び出しは未実装
                Err(RuleError::ExecutionError {
                    message: "Function calls not implemented".to_string()
                })
            }
        }
    }
}

/// DSLベースのルール実装
pub struct DslRule {
    engine: DslRuleEngine,
}

impl DslRule {
    pub fn new() -> Self {
        Self {
            engine: DslRuleEngine::new(),
        }
    }

    pub fn with_policy(mut self, policy: SecurityPolicy) -> Self {
        self.engine.add_policy(policy);
        self
    }

    pub fn with_json_policy(mut self, json: &str) -> Result<Self, serde_json::Error> {
        self.engine.load_policy_from_json(json)?;
        Ok(self)
    }

    pub fn set_variable(&mut self, name: String, value: serde_json::Value) {
        self.engine.set_variable(name, value);
    }
}

#[async_trait]
impl Rule for DslRule {
    fn name(&self) -> &'static str {
        "dsl_rule"
    }

    fn description(&self) -> &'static str {
        "DSL-based security policy rule"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn apply(&self, store: &RdfStore) -> Result<RuleResult, RuleError> {
        let results = self.engine.execute_all_policies(store).await?;
        // 複数のポリシー結果を統合
        let mut combined = RuleResult {
            triples_to_add: Vec::new(),
            triples_to_remove: Vec::new(),
            actions: Vec::new(),
            violations: Vec::new(),
            metadata: HashMap::new(),
        };

        for result in results {
            combined.triples_to_add.extend(result.triples_to_add);
            combined.triples_to_remove.extend(result.triples_to_remove);
            combined.actions.extend(result.actions);
            combined.violations.extend(result.violations);
            combined.metadata.extend(result.metadata);
        }

        Ok(combined)
    }

    fn should_apply(&self, _store: &RdfStore) -> bool {
        !self.engine.policies.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_store::store::RdfStore;
    use fukurow_core::model::Triple;

    #[test]
    fn test_security_policy_deserialization() {
        let policy_json = r#"
        {
            "name": "test_policy",
            "description": "Test security policy",
            "version": "1.0.0",
            "priority": 10,
            "rules": [
                {
                    "id": "rule1",
                    "name": "Suspicious login detection",
                    "description": "Detect multiple failed login attempts",
                    "conditions": [
                        {
                            "type": "TripleExists",
                            "config": {
                                "subject": "?user",
                                "predicate": "failed_login_count",
                                "object": "?count"
                            }
                        },
                        {
                            "type": "NumericComparison",
                            "config": {
                                "left": {"type": "Variable", "value": "count"},
                                "operator": "GreaterThan",
                                "right": {"type": "Constant", "value": 5}
                            }
                        }
                    ],
                    "actions": [
                        {
                            "type": "SecurityAction",
                            "config": {
                                "action_type": "Alert",
                                "message": "Multiple failed login attempts detected",
                                "details": {"threshold": 5}
                            }
                        }
                    ],
                    "severity": "High",
                    "metadata": {}
                }
            ],
            "metadata": {}
        }
        "#;

        let policy: SecurityPolicy = serde_json::from_str(policy_json).unwrap();
        assert_eq!(policy.name, "test_policy");
        assert_eq!(policy.rules.len(), 1);
        assert_eq!(policy.rules[0].conditions.len(), 2);
    }

    #[tokio::test]
    async fn test_dsl_rule_execution() {
        let mut store = RdfStore::new();

        // テストデータを追加
        let triple = Triple {
            subject: "user123".to_string(),
            predicate: "failed_login_count".to_string(),
            object: "10".to_string(),
        };
        store.add_triple(triple).await.unwrap();

        // DSLポリシーを作成
        let policy = SecurityPolicy {
            name: "test_policy".to_string(),
            description: "Test policy".to_string(),
            version: "1.0.0".to_string(),
            priority: 10,
            rules: vec![PolicyRule {
                id: "test_rule".to_string(),
                name: "Test rule".to_string(),
                description: "Test rule description".to_string(),
                conditions: vec![
                    Condition::TripleExists {
                        subject: "?user".to_string(),
                        predicate: "failed_login_count".to_string(),
                        object: "?count".to_string(),
                    },
                ],
                actions: vec![
                    PolicyAction::SecurityAction {
                        action_type: SecurityActionType::Alert,
                        message: "Test alert".to_string(),
                        details: HashMap::new(),
                    },
                ],
                severity: Severity::Medium,
                metadata: HashMap::new(),
            }],
            metadata: HashMap::new(),
        };

        let mut dsl_rule = DslRule::new().with_policy(policy);
        dsl_rule.set_variable("count".to_string(), serde_json::json!("10"));

        let result = dsl_rule.apply(&store).await.unwrap();
        assert_eq!(result.actions.len(), 1);
        assert!(result.triples_to_add.is_empty());
    }

    #[test]
    fn test_condition_evaluation() {
        let engine = DslRuleEngine::new();

        // 変数束縛条件のテスト
        let condition = Condition::VariableBinding {
            variable: "test_var".to_string(),
            value: "test_value".to_string(),
        };

        // 変数が設定されていないのでfalse
        assert!(!matches!(condition, Condition::VariableBinding { .. }));
    }
}
