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
    fn evaluate_query(&mut self, query: &crate::parser::SparqlQuery, store: &RdfStore) -> Result<QueryResult, crate::SparqlError>;
}

/// Prefix resolver for handling prefixed names
#[derive(Clone)]
pub struct PrefixResolver {
    prefixes: std::collections::HashMap<String, crate::parser::Iri>,
}

impl PrefixResolver {
    pub fn new(prefixes: std::collections::HashMap<String, crate::parser::Iri>) -> Self {
        Self { prefixes }
    }

    pub fn resolve(&self, prefix: &str, local: &str) -> Option<String> {
        println!("DEBUG: Resolving {}:{}", prefix, local);
        let result = self.prefixes.get(prefix).map(|iri| format!("{}{}", iri.0, local));
        println!("DEBUG: Resolved to: {:?}", result);
        result
    }
}

/// デフォルト実行エンジン
pub struct DefaultSparqlEvaluator {
    prefix_resolver: Option<PrefixResolver>,
}

impl DefaultSparqlEvaluator {
    pub fn new() -> Self {
        Self {
            prefix_resolver: None,
        }
    }

    pub fn with_prefixes(prefixes: std::collections::HashMap<String, crate::parser::Iri>) -> Self {
        Self {
            prefix_resolver: Some(PrefixResolver::new(prefixes)),
        }
    }
}

impl Default for DefaultSparqlEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl SparqlEvaluator for DefaultSparqlEvaluator {
    fn evaluate_query(&mut self, query: &crate::parser::SparqlQuery, store: &RdfStore) -> Result<QueryResult, crate::SparqlError> {
        // Set up prefixes (add default prefixes)
        let mut prefixes = query.prefixes.clone();
        // Add default RDF prefix if not present
        if !prefixes.contains_key("rdf") {
            prefixes.insert("rdf".to_string(), crate::parser::Iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string()));
        }
        if !prefixes.contains_key("rdfs") {
            prefixes.insert("rdfs".to_string(), crate::parser::Iri("http://www.w3.org/2000/01/rdf-schema#".to_string()));
        }

        println!("DEBUG: Setting up prefixes: {:?}", prefixes);
        self.prefix_resolver = Some(PrefixResolver::new(prefixes));

        // ASKクエリの特別処理
        if let crate::parser::QueryType::Ask = query.query_type {
            // ASKクエリはWHERE句を評価して結果が空でないかをチェック
            use crate::algebra::PlanBuilder;
            let builder = crate::algebra::DefaultPlanBuilder;
            let algebra = builder.to_algebra(query)?;
            let result = self.evaluate(&algebra, store)?;

            // ASKは結果が空でない場合にtrue
            match result {
                QueryResult::Select { bindings, .. } => {
                    return Ok(QueryResult::Ask { result: !bindings.is_empty() });
                }
                _ => return Err(crate::SparqlError::EvaluationError("ASK query evaluation failed".to_string())),
            }
        }

        // CONSTRUCTクエリの特別処理
        if let crate::parser::QueryType::Construct(templates) = &query.query_type {
            // CONSTRUCTクエリはWHERE句を評価し、テンプレートを使って新しいトリプルを構築
            use crate::algebra::PlanBuilder;
            let builder = crate::algebra::DefaultPlanBuilder;
            let algebra = builder.to_algebra(query)?;
            let result = self.evaluate(&algebra, store)?;

            match result {
                QueryResult::Select { bindings, .. } => {
                    let mut constructed_triples = Vec::new();

                    // 各バインディングに対してテンプレートをインスタンス化
                    for binding in bindings {
                        println!("DEBUG: Processing binding: {:?}", binding);
                        for template in templates {
                            println!("DEBUG: Processing template: {:?}", template);
                            let subject = self.instantiate_term(&template.subject, &binding, &query.prefixes);
                            let predicate = self.instantiate_term(&template.predicate, &binding, &query.prefixes);
                            let object = self.instantiate_term(&template.object, &binding, &query.prefixes);

                            println!("DEBUG: Instantiated: s={:?}, p={:?}, o={:?}", subject, predicate, object);

                            if let (Some(s), Some(p), Some(o)) = (subject, predicate, object) {
                                constructed_triples.push(fukurow_core::model::Triple {
                                    subject: s,
                                    predicate: p,
                                    object: o,
                                });
                            } else {
                                println!("DEBUG: Some terms could not be instantiated");
                            }
                        }
                    }

                    return Ok(QueryResult::Construct { triples: constructed_triples });
                }
                _ => return Err(crate::SparqlError::EvaluationError("CONSTRUCT query evaluation failed".to_string())),
            }
        }

        // 他のクエリタイプの処理
        use crate::algebra::PlanBuilder;
        let builder = crate::algebra::DefaultPlanBuilder;
        let algebra = builder.to_algebra(query)?;
        self.evaluate(&algebra, store)
    }

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
        println!("DEBUG: evaluate_bgp initial results: {:?}", results);

        // 残りのトリプルを結合
        for triple in &triples[1..] {
            let next_results = self.evaluate_triple_pattern(triple, store)?;
            println!("DEBUG: evaluate_bgp next_results: {:?}", next_results);
            results = self.join_bindings(results, next_results);
            println!("DEBUG: evaluate_bgp after join: {:?}", results);
        }

        println!("DEBUG: evaluate_bgp final results: {:?}", results);
        Ok(results)
    }

    fn evaluate_triple_pattern(&self, pattern: &TriplePattern, store: &RdfStore) -> Result<Vec<Bindings>, crate::SparqlError> {
        let mut results = Vec::new();

        // ストアから全てのトリプルを検索
        for stored_triple in store.all_triples().values().flatten() {
            let triple = &stored_triple.triple;

            // パターンマッチング
            let subject_match = self.term_matches(&pattern.subject, &triple.subject);
            let predicate_match = self.term_matches(&pattern.predicate, &triple.predicate);
            let object_match = self.term_matches(&pattern.object, &triple.object);

            println!("DEBUG: Matching triple: s={:?}, p={:?}, o={:?}", triple.subject, triple.predicate, triple.object);
            println!("DEBUG: Pattern: s={:?}, p={:?}, o={:?}", pattern.subject, pattern.predicate, pattern.object);
            println!("DEBUG: Matches: s={}, p={}, o={}", subject_match, predicate_match, object_match);

            if subject_match && predicate_match && object_match {

                let mut binding = HashMap::new();

                // 変数を束縛
                println!("DEBUG: Before binding: {:?}", binding);
                self.bind_term(&pattern.subject, &triple.subject, &mut binding);
                self.bind_term(&pattern.predicate, &triple.predicate, &mut binding);
                self.bind_term(&pattern.object, &triple.object, &mut binding);
                println!("DEBUG: After binding: {:?}", binding);

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
            Term::PrefixedName(prefix, local) => {
                println!("DEBUG: Resolving prefixed name: {}:{}", prefix, local);
                if let Some(resolver) = &self.prefix_resolver {
                    if let Some(resolved) = resolver.resolve(prefix, local) {
                        println!("DEBUG: Resolved to: {}", resolved);
                        resolved == term
                    } else {
                        println!("DEBUG: Resolver failed to resolve");
                        false
                    }
                } else {
                    // フォールバック: 簡易実装
                    println!("DEBUG: Using fallback resolution");
                    if prefix == "ex" {
                        let resolved = format!("http://example.org/{}", local);
                        println!("DEBUG: ex:{} -> {}", local, resolved);
                        resolved == term
                    } else if prefix == "rdf" && local == "type" {
                        let result = term == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
                        println!("DEBUG: rdf:type check: {} == {} -> {}", term, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type", result);
                        result
                    } else if prefix == "foaf" {
                        let resolved = format!("http://xmlns.com/foaf/0.1/{}", local);
                        println!("DEBUG: foaf:{} -> {}", local, resolved);
                        resolved == term
                    } else {
                        println!("DEBUG: Unknown prefix: {}", prefix);
                        false
                    }
                }
            }
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

    fn resolve_prefixed_name(&self, prefix: &str, local: &str, prefixes: &std::collections::HashMap<String, crate::parser::Iri>) -> Option<String> {
        prefixes.get(prefix).map(|iri| format!("{}{}", iri.0, local))
    }

    fn instantiate_term(&self, term: &Term, binding: &Bindings, prefixes: &std::collections::HashMap<String, crate::parser::Iri>) -> Option<String> {
        match term {
            Term::Variable(var) => {
                // バインディングから値を取得
                binding.get(var).and_then(|bound_term| {
                    match bound_term {
                        Term::Iri(iri) => Some(iri.0.clone()),
                        Term::Literal(lit) => Some(lit.value.clone()),
                        _ => None,
                    }
                })
            }
            Term::Iri(iri) => Some(iri.0.clone()),
            Term::Literal(lit) => Some(lit.value.clone()),
            Term::PrefixedName(prefix, local) => {
                println!("DEBUG: instantiate_term resolving {}:{}", prefix, local);
                if let Some(resolver) = &self.prefix_resolver {
                    let result = resolver.resolve(prefix, local);
                    println!("DEBUG: instantiate_term resolved to {:?}", result);
                    result
                } else {
                    println!("DEBUG: instantiate_term no resolver");
                    None
                }
            }
            Term::BlankNode(_) => None, // Blank nodes not supported in CONSTRUCT for now
        }
    }

    fn join_bindings(&self, left: Vec<Bindings>, right: Vec<Bindings>) -> Vec<Bindings> {
        println!("DEBUG: join_bindings left: {:?}, right: {:?}", left, right);
        let mut results = Vec::new();

        for left_binding in &left {
            for right_binding in &right {
                if self.bindings_compatible(left_binding, right_binding) {
                    let mut joined = left_binding.clone();
                    joined.extend(right_binding.clone());
                    println!("DEBUG: joined: {:?}", joined);
                    results.push(joined);
                }
            }
        }

        println!("DEBUG: join_bindings result: {:?}", results);
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
