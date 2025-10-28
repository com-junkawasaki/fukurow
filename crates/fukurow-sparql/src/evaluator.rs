//! SPARQL 実行エンジン

use crate::algebra::Algebra;
use crate::parser::{Bindings, TriplePattern, Term, Variable, VarOrIri, Expression, OrderCondition};
use fukurow_store::store::RdfStore;
use fukurow_core::model::Triple;
use std::collections::{HashMap, HashSet};
use itertools::Itertools;
use crate::SparqlError;

/// クエリ結果
#[derive(Debug, Clone)]
pub enum QueryResult {
    Select { variables: Vec<Variable>, bindings: Vec<Bindings> },
    Construct { triples: Vec<Triple> },
    Ask { result: bool },
    Describe { triples: Vec<Triple> },
}

/// 実行エンジントレイト
pub trait SparqlEvaluator {
    fn evaluate(&self, algebra: &Algebra, store: &RdfStore) -> Result<QueryResult, crate::SparqlError>;
}

/// デフォルト実行エンジン
pub struct DefaultSparqlEvaluator;

impl SparqlEvaluator for DefaultSparqlEvaluator {
    fn evaluate(&self, algebra: &Algebra, store: &RdfStore) -> Result<QueryResult, crate::SparqlError> {
        match algebra {
            Algebra::Bgp(triples) => {
                let bindings = self.evaluate_bgp(triples, store)?;
                Ok(QueryResult::Select {
                    variables: self.extract_variables(triples),
                    bindings,
                })
            }
            Algebra::Project(inner, vars) => {
                let mut result = self.evaluate(inner, store)?;
                if let QueryResult::Select { bindings, .. } = &mut result {
                    // 投影変数のみ保持
                    for binding in bindings {
                        let keys: Vec<_> = binding.keys().cloned().collect();
                        for key in keys {
                            if !vars.contains(&key) {
                                binding.remove(&key);
                            }
                        }
                    }
                }
                Ok(result)
            }
            Algebra::Filter(inner, expr) => {
                let mut result = self.evaluate(inner, store)?;
                if let QueryResult::Select { bindings, .. } = &mut result {
                    bindings.retain(|binding| self.evaluate_expression(expr, binding));
                }
                Ok(result)
            }
            Algebra::Slice { input, offset, limit } => {
                let mut result = self.evaluate(input, store)?;
                if let QueryResult::Select { bindings, .. } = &mut result {
                    let start = offset.unwrap_or(0) as usize;
                    let end = start + limit.unwrap_or(bindings.len() as u64) as usize;
                    *bindings = bindings[start..end.min(bindings.len())].to_vec();
                }
                Ok(result)
            }
            Algebra::OrderBy(inner, order_conditions) => {
                let mut result = self.evaluate(inner, store)?;
                if let QueryResult::Select { bindings, .. } = &mut result {
                    bindings.sort_by(|a, b| {
                        for condition in order_conditions {
                            let cmp = self.compare_bindings(a, b, condition);
                            if cmp != std::cmp::Ordering::Equal {
                                return cmp;
                            }
                        }
                        std::cmp::Ordering::Equal
                    });
                }
                Ok(result)
            }
            Algebra::Union(left, right) => {
                let left_result = self.evaluate(left, store)?;
                let right_result = self.evaluate(right, store)?;

                match (left_result, right_result) {
                    (QueryResult::Select { variables: left_vars, bindings: left_bindings },
                     QueryResult::Select { variables: right_vars, bindings: right_bindings }) => {
                        // For UNION, combine variables and bindings
                        let mut all_vars = left_vars.clone();
                        for var in &right_vars {
                            if !all_vars.contains(var) {
                                all_vars.push(var.clone());
                            }
                        }

                        let mut all_bindings = left_bindings;
                        all_bindings.extend(right_bindings);

                        Ok(QueryResult::Select {
                            variables: all_vars,
                            bindings: all_bindings,
                        })
                    }
                    _ => Err(SparqlError::EvaluationError("UNION only supported for SELECT results".to_string())),
                }
            }
            Algebra::LeftJoin { left, right, expr } => {
                let left_result = self.evaluate(left, store)?;
                let right_result = self.evaluate(right, store)?;

                match (left_result, right_result) {
                    (QueryResult::Select { variables: left_vars, bindings: mut left_bindings },
                     QueryResult::Select { variables: right_vars, bindings: right_bindings }) => {
                        // For LEFT JOIN (OPTIONAL), extend left bindings with matching right bindings
                        for left_binding in &mut left_bindings {
                            let mut extended = false;
                            for right_binding in &right_bindings {
                                // Check if bindings are compatible (same values for common variables)
                                let compatible = left_vars.iter().all(|var| {
                                    if right_vars.contains(var) {
                                        left_binding.get(var) == right_binding.get(var)
                                    } else {
                                        true
                                    }
                                });

                                if compatible {
                                    // Extend left binding with right binding
                                    for (var, value) in right_binding {
                                        left_binding.insert(var.clone(), value.clone());
                                    }
                                    extended = true;
                                    break; // For now, take first match
                                }
                            }

                            if !extended {
                                // No match found, add unbound variables from right
                                for var in &right_vars {
                                    if !left_binding.contains_key(var) {
                                        // Leave unbound (not in binding)
                                    }
                                }
                            }
                        }

                        // Combine variables
                        let mut all_vars = left_vars;
                        for var in right_vars {
                            if !all_vars.contains(&var) {
                                all_vars.push(var);
                            }
                        }

                        Ok(QueryResult::Select {
                            variables: all_vars,
                            bindings: left_bindings,
                        })
                    }
                    _ => Err(SparqlError::EvaluationError("LEFT JOIN only supported for SELECT results".to_string())),
                }
            }
            Algebra::Distinct(inner) => {
                let mut result = self.evaluate(inner, store)?;
                if let QueryResult::Select { bindings, .. } = &mut result {
                    let mut seen = Vec::new();
                    bindings.retain(|binding| {
                        if seen.contains(binding) {
                            false
                        } else {
                            seen.push(binding.clone());
                            true
                        }
                    });
                }
                Ok(result)
            }
            Algebra::Reduced(inner) => {
                // REDUCED は DISTINCT と同様に扱う（実装簡略化）
                self.evaluate(&Algebra::Distinct(inner.clone()), store)
            }
            // TODO: 他の代数演算子の実装
            _ => Err(SparqlError::UnsupportedFeature("Algebra operator not implemented".to_string())),
        }
    }
}

impl DefaultSparqlEvaluator {
    fn evaluate_bgp(&self, triples: &[TriplePattern], store: &RdfStore) -> Result<Vec<Bindings>, crate::SparqlError> {
        if triples.is_empty() {
            return Ok(vec![HashMap::new()]);
        }

        // 最初のトリプルを評価
        let mut results = self.evaluate_triple_pattern(&triples[0], store)?;

        // 残りのトリプルを結合
        for triple in &triples[1..] {
            let next_results = self.evaluate_triple_pattern(triple, store)?;
            results = self.join_bindings(results, next_results);
        }

        Ok(results)
    }

    fn evaluate_triple_pattern(&self, pattern: &TriplePattern, store: &RdfStore) -> Result<Vec<Bindings>, crate::SparqlError> {
        let mut results = Vec::new();

        // ストアから全てのトリプルを検索
        for stored_triple in store.all_triples().values().flatten() {
            let triple = &stored_triple.triple;

            // パターンマッチング
            if self.term_matches(&pattern.subject, &triple.subject) &&
               self.term_matches(&pattern.predicate, &triple.predicate) &&
               self.term_matches(&pattern.object, &triple.object) {

                let mut binding = HashMap::new();

                // 変数を束縛
                self.bind_term(&pattern.subject, &triple.subject, &mut binding);
                self.bind_term(&pattern.predicate, &triple.predicate, &mut binding);
                self.bind_term(&pattern.object, &triple.object, &mut binding);

                results.push(binding);
            }
        }

        Ok(results)
    }

    fn term_matches(&self, pattern: &Term, term: &str) -> bool {
        match pattern {
            Term::Variable(_) => true, // 変数は常にマッチ
            Term::Iri(pattern_iri) => {
                &pattern_iri.0 == term
            }
            Term::Literal(pattern_lit) => {
                &pattern_lit.value == term
            }
            Term::BlankNode(_) => true, // TODO: ブランクノード比較
            Term::PrefixedName(_, _) => false, // TODO: prefix解決
        }
    }

    fn bind_term(&self, pattern: &Term, term: &str, binding: &mut Bindings) {
        if let Term::Variable(var) = pattern {
            // String を crate::parser::Term に変換
            // TODO: IRI, Literal, BlankNode の区別を実装
            let term_value = Term::Iri(crate::parser::Iri(term.to_string()));
            binding.insert(var.clone(), term_value);
        }
    }

    fn join_bindings(&self, left: Vec<Bindings>, right: Vec<Bindings>) -> Vec<Bindings> {
        let mut results = Vec::new();

        for left_binding in &left {
            for right_binding in &right {
                if self.bindings_compatible(left_binding, right_binding) {
                    let mut joined = left_binding.clone();
                    joined.extend(right_binding.clone());
                    results.push(joined);
                }
            }
        }

        results
    }

    fn bindings_compatible(&self, left: &Bindings, right: &Bindings) -> bool {
        for (var, left_term) in left {
            if let Some(right_term) = right.get(var) {
                if left_term != right_term {
                    return false;
                }
            }
        }
        true
    }

    fn merge_bindings(&self, left: &Bindings, right: &Bindings) -> Bindings {
        let mut merged = left.clone();
        merged.extend(right.clone());
        merged
    }

    fn evaluate_expression(&self, expr: &Expression, binding: &Bindings) -> bool {
        match expr {
            Expression::Variable(var) => binding.contains_key(var),
            Expression::Bound(var) => binding.contains_key(var),
            Expression::Not(inner) => !self.evaluate_expression(inner, binding),
            Expression::And(left, right) => {
                self.evaluate_expression(left, binding) && self.evaluate_expression(right, binding)
            }
            Expression::Or(left, right) => {
                self.evaluate_expression(left, binding) || self.evaluate_expression(right, binding)
            }
            Expression::Equal(left, right) => {
                self.compare_expressions(left, right, binding) == std::cmp::Ordering::Equal
            }
            // TODO: 他の式評価の実装
            _ => true, // デフォルトでtrue
        }
    }

    fn compare_expressions(&self, left: &Expression, right: &Expression, binding: &Bindings) -> std::cmp::Ordering {
        // 簡易的な比較（TODO: 完全実装）
        match (left, right) {
            (Expression::Variable(var1), Expression::Variable(var2)) => {
                match (binding.get(var1), binding.get(var2)) {
                    (Some(term1), Some(term2)) => self.compare_terms(term1, term2),
                    _ => std::cmp::Ordering::Equal,
                }
            }
            _ => std::cmp::Ordering::Equal,
        }
    }

    fn compare_terms(&self, left: &Term, right: &Term) -> std::cmp::Ordering {
        match (left, right) {
            (Term::Iri(iri1), Term::Iri(iri2)) => iri1.to_string().cmp(&iri2.to_string()),
            (Term::Literal(lit1), Term::Literal(lit2)) => lit1.value.cmp(&lit2.value),
            _ => std::cmp::Ordering::Equal,
        }
    }

    fn compare_bindings(&self, left: &Bindings, right: &Bindings, condition: &OrderCondition) -> std::cmp::Ordering {
        // TODO: OrderCondition の評価実装
        std::cmp::Ordering::Equal
    }

    fn extract_variables(&self, triples: &[TriplePattern]) -> Vec<Variable> {
        let mut vars = HashSet::new();

        for triple in triples {
            self.extract_vars_from_term(&triple.subject, &mut vars);
            self.extract_vars_from_term(&triple.predicate, &mut vars);
            self.extract_vars_from_term(&triple.object, &mut vars);
        }

        vars.into_iter().collect()
    }

    fn extract_vars_from_term(&self, term: &Term, vars: &mut HashSet<Variable>) {
        if let Term::Variable(var) = term {
            vars.insert(var.clone());
        }
    }
}
