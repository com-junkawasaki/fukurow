//! SPARQL クエリ最適化

use crate::algebra::Algebra;
use crate::parser::{Expression, Variable, TriplePattern, Term};
use std::collections::HashMap;
use crate::SparqlError;

/// 最適化ルール
#[derive(Debug, Clone)]
pub enum OptimizationRule {
    /// BGP 順序付け (選択度に基づく)
    BgpReordering,

    /// Filter プッシュダウン
    FilterPushDown,

    /// Join 順序付け
    JoinReordering,

    /// 定数畳み込み
    ConstantFolding,

    /// 不要な投影除去
    ProjectionElimination,

    /// Union 最適化
    UnionOptimization,

    /// 空パターン除去
    EmptyPatternElimination,
}

/// 最適化統計
#[derive(Debug, Clone)]
pub struct QueryStats {
    pub triple_count: usize,
    pub variable_count: usize,
    pub selectivity_estimates: HashMap<String, f64>,
    pub predicate_selectivities: HashMap<String, f64>,
}

/// 最適化器トレイト
pub trait SparqlOptimizer {
    fn optimize(&self, algebra: Algebra, stats: Option<&QueryStats>) -> Algebra;
}

/// デフォルト最適化器
pub struct DefaultSparqlOptimizer {
    rules: Vec<OptimizationRule>,
}

impl Default for DefaultSparqlOptimizer {
    fn default() -> Self {
        Self {
            rules: vec![
                OptimizationRule::EmptyPatternElimination,
                OptimizationRule::FilterPushDown,
                OptimizationRule::BgpReordering,
                OptimizationRule::ConstantFolding,
                OptimizationRule::ProjectionElimination,
                OptimizationRule::UnionOptimization,
            ],
        }
    }
}

impl SparqlOptimizer for DefaultSparqlOptimizer {
    fn optimize(&self, algebra: Algebra, stats: Option<&QueryStats>) -> Algebra {
        let mut optimized = algebra;

        for rule in &self.rules {
            optimized = self.apply_rule(optimized, rule, stats);
        }

        optimized
    }
}

impl DefaultSparqlOptimizer {
    fn apply_rule(&self, algebra: Algebra, rule: &OptimizationRule, stats: Option<&QueryStats>) -> Algebra {
        match rule {
            OptimizationRule::FilterPushDown => self.push_down_filters(algebra),
            OptimizationRule::BgpReordering => self.reorder_bgp(algebra, stats),
            OptimizationRule::ConstantFolding => self.fold_constants(algebra),
            OptimizationRule::ProjectionElimination => self.eliminate_projections(algebra),
            OptimizationRule::UnionOptimization => self.optimize_union(algebra),
            OptimizationRule::EmptyPatternElimination => self.eliminate_empty_patterns(algebra),
            _ => algebra,
        }
    }

    fn push_down_filters(&self, algebra: Algebra) -> Algebra {
        match algebra {
            Algebra::Filter(inner, expr) => {
                // フィルタを下位にプッシュ
                match *inner {
                    Algebra::LeftJoin { left, right, expr: join_expr } => {
                        // OPTIONAL の場合、フィルタを右側にプッシュ
                        Algebra::LeftJoin {
                            left,
                            right: Box::new(Algebra::Filter(right, expr)),
                            expr: join_expr,
                        }
                    }
                    Algebra::Union(left, right) => {
                        // UNION の場合、両側にプッシュ
                        Algebra::Union(
                            Box::new(Algebra::Filter(left, expr.clone())),
                            Box::new(Algebra::Filter(right, expr)),
                        )
                    }
                    Algebra::Graph(graph, inner) => {
                        // GRAPH の場合、内側にプッシュ
                        Algebra::Graph(graph, Box::new(Algebra::Filter(inner, expr)))
                    }
                    _ => Algebra::Filter(inner, expr),
                }
            }
            _ => algebra,
        }
    }

    fn reorder_bgp(&self, algebra: Algebra, stats: Option<&QueryStats>) -> Algebra {
        match algebra {
            Algebra::Bgp(mut triples) => {
                // BGP 内のトリプルパターンを選択度で並べ替え
                if let Some(stats) = stats {
                    triples.sort_by(|a, b| {
                        let selectivity_a = self.estimate_triple_selectivity(a, stats);
                        let selectivity_b = self.estimate_triple_selectivity(b, stats);
                        selectivity_a.partial_cmp(&selectivity_b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                Algebra::Bgp(triples)
            }
            _ => algebra,
        }
    }

    fn estimate_triple_selectivity(&self, triple: &TriplePattern, stats: &QueryStats) -> f64 {
        let mut selectivity = 1.0;

        // Subject の選択度
        if let Term::Iri(_) = &triple.subject {
            selectivity *= 0.1; // IRI指定で10分の1
        }

        // Predicate の選択度
        if let Term::Iri(iri) = &triple.predicate {
            let pred_key = iri.to_string();
            selectivity *= stats.predicate_selectivities.get(&pred_key).unwrap_or(&0.5);
        }

        // Object の選択度
        if let Term::Iri(_) = &triple.object {
            selectivity *= 0.1;
        } else if let Term::Literal(_) = &triple.object {
            selectivity *= 0.2;
        }

        selectivity
    }

    fn fold_constants(&self, algebra: Algebra) -> Algebra {
        match algebra {
            Algebra::Filter(inner, expr) => {
                let folded_expr = self.fold_expression_constants(expr);
                Algebra::Filter(inner, folded_expr)
            }
            Algebra::Extend(inner, var, expr) => {
                let folded_expr = self.fold_expression_constants(expr);
                Algebra::Extend(inner, var, folded_expr)
            }
            _ => algebra,
        }
    }

    fn fold_expression_constants(&self, expr: Expression) -> Expression {
        match expr {
            Expression::Add(left, right) => {
                match (*left.clone(), *right.clone()) {
                    (Expression::Literal(l), Expression::Literal(r)) => {
                        // TODO: リテラル演算の実装
                        Expression::Add(Box::new(Expression::Literal(l)), Box::new(Expression::Literal(r)))
                    }
                    _ => Expression::Add(left, right),
                }
            }
            // TODO: 他の定数畳み込み
            _ => expr,
        }
    }

    fn eliminate_projections(&self, algebra: Algebra) -> Algebra {
        // 不要な投影の除去
        match algebra {
            Algebra::Project(inner, vars) => {
                match *inner {
                    Algebra::Project(inner2, _) => {
                        // 連続した投影を統合
                        Algebra::Project(inner2, vars)
                    }
                    _ => Algebra::Project(inner, vars),
                }
            }
            _ => algebra,
        }
    }

    fn optimize_union(&self, algebra: Algebra) -> Algebra {
        match algebra {
            Algebra::Union(left, right) => {
                // 同じパターンの UNION を除去
                if left == right {
                    *left
                } else {
                    Algebra::Union(left, right)
                }
            }
            _ => algebra,
        }
    }

    fn eliminate_empty_patterns(&self, algebra: Algebra) -> Algebra {
        match algebra {
            Algebra::Bgp(triples) => {
                if triples.is_empty() {
                    // 空の BGP はユニットとして扱う
                    Algebra::Bgp(vec![])
                } else {
                    Algebra::Bgp(triples)
                }
            }
            Algebra::Filter(inner, _) => {
                let optimized_inner = self.eliminate_empty_patterns(*inner);
                if let Algebra::Bgp(triples) = &optimized_inner {
                    if triples.is_empty() {
                        // 空パターンにフィルタを適用しても空
                        optimized_inner
                    } else {
                        Algebra::Filter(Box::new(optimized_inner), Expression::Variable(Variable("dummy".to_string())))
                    }
                } else {
                    Algebra::Filter(Box::new(optimized_inner), Expression::Variable(Variable("dummy".to_string())))
                }
            }
            _ => algebra,
        }
    }
}
