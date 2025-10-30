//! SPARQL 論理代数

use crate::parser::{SparqlQuery, GraphPattern, TriplePattern, Expression, VarOrIri, OrderCondition, Bindings, Variable, QueryType};
use crate::SparqlError;

/// 論理代数演算子
#[derive(Debug, Clone, PartialEq)]
pub enum Algebra {
    /// Basic Graph Pattern
    Bgp(Vec<TriplePattern>),

    /// Left Join (OPTIONAL)
    LeftJoin {
        left: Box<Algebra>,
        right: Box<Algebra>,
        expr: Option<Expression>,
    },

    /// Union
    Union(Box<Algebra>, Box<Algebra>),

    /// Filter
    Filter(Box<Algebra>, Expression),

    /// Projection
    Project(Box<Algebra>, Vec<Variable>),

    /// Extend (AS)
    Extend(Box<Algebra>, Variable, Expression),

    /// Slice (LIMIT/OFFSET)
    Slice {
        input: Box<Algebra>,
        offset: Option<u64>,
        limit: Option<u64>,
    },

    /// Order By
    OrderBy(Box<Algebra>, Vec<OrderCondition>),

    /// Distinct
    Distinct(Box<Algebra>),

    /// Reduced
    Reduced(Box<Algebra>),

    /// Group By with aggregates
    Group {
        input: Box<Algebra>,
        keys: Vec<Expression>,
        aggs: Vec<Aggregate>,
    },

    /// Graph
    Graph(VarOrIri, Box<Algebra>),

    /// Minus
    Minus(Box<Algebra>, Box<Algebra>),

    /// Service (federated query)
    Service(VarOrIri, Box<Algebra>, bool), // silent

    /// Values
    Values(Vec<Bindings>),
}

/// Aggregate function
#[derive(Debug, Clone, PartialEq)]
pub enum Aggregate {
    Count { expr: Option<Box<Expression>>, distinct: bool },
    Sum(Box<Expression>, bool),
    Avg(Box<Expression>, bool),
    Min(Box<Expression>, bool),
    Max(Box<Expression>, bool),
    GroupConcat { expr: Box<Expression>, distinct: bool, separator: Option<String> },
    Sample(Box<Expression>),
}

/// Plan builder trait
pub trait PlanBuilder {
    fn to_algebra(&self, query: &SparqlQuery) -> Result<Algebra, crate::SparqlError>;
}

/// Default algebra builder
pub struct DefaultPlanBuilder;

impl PlanBuilder for DefaultPlanBuilder {
    fn to_algebra(&self, query: &SparqlQuery) -> Result<Algebra, crate::SparqlError> {
        let mut algebra = self.graph_pattern_to_algebra(&query.where_clause)?;

        // Apply VALUES
        if let Some(values) = &query.values {
            algebra = Algebra::Union(
                Box::new(algebra),
                Box::new(Algebra::Values(values.clone())),
            );
        }

        // Apply solution modifiers
        if let Some(limit) = query.solution_modifier.limit {
            algebra = Algebra::Slice {
                input: Box::new(algebra),
                offset: query.solution_modifier.offset,
                limit: Some(limit),
            };
        } else if query.solution_modifier.offset.is_some() {
            algebra = Algebra::Slice {
                input: Box::new(algebra),
                offset: query.solution_modifier.offset,
                limit: None,
            };
        }

        if !query.solution_modifier.order.is_none() {
            if let Some(order) = &query.solution_modifier.order {
                algebra = Algebra::OrderBy(
                    Box::new(algebra),
                    order.clone(),
                );
            }
        }

        if query.solution_modifier.distinct {
            algebra = Algebra::Distinct(Box::new(algebra));
        }

        if query.solution_modifier.reduced {
            algebra = Algebra::Reduced(Box::new(algebra));
        }

        // Apply GROUP BY and aggregates
        if let Some(group_keys) = &query.solution_modifier.group {
            let aggs = self.extract_aggregates(&query.variables)?;
            algebra = Algebra::Group {
                input: Box::new(algebra),
                keys: group_keys.clone(),
                aggs,
            };
        }

        // Projection for SELECT
        match &query.query_type {
            QueryType::Select => {
                algebra = Algebra::Project(Box::new(algebra), query.variables.clone());
            }
            QueryType::Construct(_) => {
                // CONSTRUCT: no projection, keep all variables
            }
            QueryType::Ask => {
                // ASK: no projection needed
            }
            QueryType::Describe(_) => {
                // TODO: DESCRIBE処理
            }
        }

        Ok(algebra)
    }
}

impl DefaultPlanBuilder {
    fn graph_pattern_to_algebra(&self, pattern: &GraphPattern) -> Result<Algebra, crate::SparqlError> {
        match pattern {
            GraphPattern::Bgp(triples) => Ok(Algebra::Bgp(triples.clone())),
            GraphPattern::Optional(inner) => {
                let inner_alg = self.graph_pattern_to_algebra(inner)?;
                Ok(Algebra::LeftJoin {
                    left: Box::new(Algebra::Bgp(vec![])), // TODO: 適切なleft plan
                    right: Box::new(inner_alg),
                    expr: None,
                })
            }
            GraphPattern::Union(patterns) => {
                if patterns.is_empty() {
                    return Ok(Algebra::Bgp(vec![]));
                }

                let mut result = self.graph_pattern_to_algebra(&patterns[0])?;
                for pattern in &patterns[1..] {
                    let right = self.graph_pattern_to_algebra(pattern)?;
                    result = Algebra::Union(Box::new(result), Box::new(right));
                }
                Ok(result)
            }
            GraphPattern::Filter(expr, inner) => {
                let inner_alg = self.graph_pattern_to_algebra(inner)?;
                Ok(Algebra::Filter(Box::new(inner_alg), expr.clone()))
            }
            GraphPattern::Graph(graph, inner) => {
                let inner_alg = self.graph_pattern_to_algebra(inner)?;
                Ok(Algebra::Graph(graph.clone(), Box::new(inner_alg)))
            }
            GraphPattern::Minus(left, right) => {
                let left_alg = self.graph_pattern_to_algebra(left)?;
                let right_alg = self.graph_pattern_to_algebra(right)?;
                Ok(Algebra::Minus(Box::new(left_alg), Box::new(right_alg)))
            }
            GraphPattern::Service(endpoint, inner, silent) => {
                let inner_alg = self.graph_pattern_to_algebra(inner)?;
                Ok(Algebra::Service(endpoint.clone(), Box::new(inner_alg), *silent))
            }
        }
    }

    fn extract_aggregates(&self, variables: &[Variable]) -> Result<Vec<Aggregate>, crate::SparqlError> {
        // TODO: 変数から集約関数を抽出する実装
        // 現在はダミー実装
        Ok(vec![])
    }
}
