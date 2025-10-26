//! SPARQL 1.1 構文解析器

use logos::Logos;
use winnow::{
    ModalResult, Parser,
    combinator::{alt, opt, repeat, separated, preceded, terminated, delimited},
    token::take_while,
};
use std::collections::HashMap;

/// SPARQL Parser trait
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Iri(pub String);

impl std::fmt::Display for Iri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// RDF Literal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Literal {
    pub value: String,
    pub datatype: Option<Iri>,
    pub language: Option<String>,
}

/// RDF Variable
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Variable(pub String);

/// SPARQL トークン
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // whitespace
pub enum Token<'a> {
    #[token("SELECT")]
    Select,

    #[token("CONSTRUCT")]
    Construct,

    #[token("ASK")]
    Ask,

    #[token("DESCRIBE")]
    Describe,

    #[token("WHERE")]
    Where,

    #[token("FILTER")]
    Filter,

    #[token("OPTIONAL")]
    Optional,

    #[token("UNION")]
    Union,

    #[token("GRAPH")]
    Graph,

    #[token("ORDER")]
    Order,

    #[token("BY")]
    By,

    #[token("LIMIT")]
    Limit,

    #[token("OFFSET")]
    Offset,

    #[token("DISTINCT")]
    Distinct,

    #[token("REDUCED")]
    Reduced,

    #[token("FROM")]
    From,

    #[token("NAMED")]
    Named,

    #[regex(r"\?[a-zA-Z_][a-zA-Z0-9_]*")]
    Variable(&'a str),

    #[regex(r"<[^>]*>")]
    Iri(&'a str),

    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLiteral(&'a str),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*:[a-zA-Z_][a-zA-Z0-9_]*")]
    PrefixedName(&'a str),

    #[regex(r"[0-9]+")]
    Integer(&'a str),

    #[regex(r"[0-9]*\.[0-9]+")]
    Decimal(&'a str),

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token(".")]
    Dot,

    #[token(",")]
    Comma,

    #[token(";")]
    Semicolon,

    #[token(":")]
    Colon,

    #[token("^^")]
    DoubleCaret,

    #[token("@")]
    At,

    #[token("a")]
    A,

    #[token("^")]
    Caret,

    #[token("/")]
    Slash,

    #[token("|")]
    Pipe,

    #[token("*")]
    Star,

    #[token("+")]
    Plus,

    #[token("?")]
    Question,

    #[token("=")]
    Equals,

    #[token("!=")]
    NotEquals,

    #[token("<")]
    LessThan,

    #[token("<=")]
    LessEqual,

    #[token(">")]
    GreaterThan,

    #[token(">=")]
    GreaterEqual,

    #[token("&&")]
    And,

    #[token("||")]
    Or,

    #[token("!")]
    Not,

    #[token("ASC")]
    Asc,

    #[token("DESC")]
    Desc,

    #[token("COUNT")]
    Count,

    #[token("SUM")]
    Sum,

    #[token("AVG")]
    Avg,

    #[token("MIN")]
    Min,

    #[token("MAX")]
    Max,

    #[token("GROUP")]
    Group,

    #[token("HAVING")]
    Having,

    #[token("AS")]
    As,

    #[token("BASE")]
    Base,

    #[token("PREFIX")]
    Prefix,
}

/// RDF Term
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    Iri(Iri),
    Literal(Literal),
    Variable(Variable),
    BlankNode(String),
}

/// Triple Pattern
#[derive(Debug, Clone, PartialEq)]
pub struct TriplePattern {
    pub subject: Term,
    pub predicate: Term,
    pub object: Term,
}

/// Property Path
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyPath {
    Predicate(Iri),
    Inverse(Box<PropertyPath>),
    Sequence(Vec<PropertyPath>),
    Alternative(Vec<PropertyPath>),
    ZeroOrMore(Box<PropertyPath>),
    OneOrMore(Box<PropertyPath>),
    ZeroOrOne(Box<PropertyPath>),
}

/// Graph Pattern
#[derive(Debug, Clone, PartialEq)]
pub enum GraphPattern {
    Bgp(Vec<TriplePattern>),
    Optional(Box<GraphPattern>),
    Union(Vec<GraphPattern>),
    Filter(Expression, Box<GraphPattern>),
    Graph(VarOrIri, Box<GraphPattern>),
    Minus(Box<GraphPattern>, Box<GraphPattern>),
    Service(VarOrIri, Box<GraphPattern>, bool), // silent flag
}

/// Expression
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Variable(Variable),
    Iri(Iri),
    Literal(Literal),
    // Arithmetic
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    // Comparison
    Equal(Box<Expression>, Box<Expression>),
    NotEqual(Box<Expression>, Box<Expression>),
    LessThan(Box<Expression>, Box<Expression>),
    LessThanOrEqual(Box<Expression>, Box<Expression>),
    GreaterThan(Box<Expression>, Box<Expression>),
    GreaterThanOrEqual(Box<Expression>, Box<Expression>),
    // Logical
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
    // Functions
    Bound(Variable),
    IsIri(Box<Expression>),
    IsLiteral(Box<Expression>),
    IsBlank(Box<Expression>),
    Str(Box<Expression>),
    Lang(Box<Expression>),
    Datatype(Box<Expression>),
    IriFunc(Box<Expression>), // Rename to avoid conflict
    Uri(Box<Expression>),
    Bnode(Box<Expression>),
    // Regex
    Regex(Box<Expression>, Box<Expression>, Option<Box<Expression>>),
    // Exists
    Exists(Box<GraphPattern>),
    NotExists(Box<GraphPattern>),
}

/// Var or IRI
#[derive(Debug, Clone, PartialEq)]
pub enum VarOrIri {
    Var(Variable),
    Iri(Iri),
}

/// Order condition
#[derive(Debug, Clone, PartialEq)]
pub enum OrderCondition {
    Asc(Expression),
    Desc(Expression),
}

/// Solution modifier
#[derive(Debug, Clone, PartialEq)]
pub struct SolutionModifier {
    pub group: Option<Vec<Expression>>,
    pub having: Option<Vec<Expression>>,
    pub order: Option<Vec<OrderCondition>>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub distinct: bool,
    pub reduced: bool,
}

/// SPARQL Query
#[derive(Debug, Clone, PartialEq)]
pub struct SparqlQuery {
    pub query_type: QueryType,
    pub variables: Vec<Variable>,
    pub dataset: Vec<GraphRef>,
    pub where_clause: GraphPattern,
    pub solution_modifier: SolutionModifier,
    pub values: Option<Vec<Bindings>>,
    pub base_iri: Option<Iri>,
    pub prefixes: HashMap<String, Iri>,
}

/// Query Type
#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    Select,
    Construct(Vec<TriplePattern>),
    Ask,
    Describe(Vec<VarOrIri>),
}

/// Bindings (variable -> term mapping)
pub type Bindings = HashMap<Variable, Term>;

/// Graph reference for FROM/FROM NAMED
#[derive(Debug, Clone, PartialEq)]
pub enum GraphRef {
    Named(Iri),
    Default(Iri),
}

use crate::SparqlError;

/// SPARQL Parser trait
pub trait SparqlParser {
    fn parse(&self, query: &str) -> Result<SparqlQuery, SparqlError>;
}

/// Default implementation
pub struct DefaultSparqlParser;

impl SparqlParser for DefaultSparqlParser {
    fn parse(&self, query: &str) -> Result<SparqlQuery, SparqlError> {
        // 簡易パーサー実装 - 基本的なSELECTクエリのみ
        let mut prefixes = HashMap::new();
        let mut variables = Vec::new();
        let mut query_type = QueryType::Select;

        // 基本的なPREFIXとSELECTのパース
        let lines: Vec<&str> = query.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

        for line in &lines {
            if line.starts_with("PREFIX") {
                // PREFIX解析 (簡易)
                if let Some(prefix_def) = line.strip_prefix("PREFIX") {
                    let parts: Vec<&str> = prefix_def.trim().split_whitespace().collect();
                    if parts.len() >= 2 {
                        let prefix = parts[0].trim_end_matches(':');
                        let iri = parts[1].trim_matches('<').trim_matches('>');
                        prefixes.insert(prefix.to_string(), Iri(iri.to_string()));
                    }
                }
            } else if line.starts_with("SELECT") {
                // SELECT変数解析 (簡易)
                if let Some(var_part) = line.strip_prefix("SELECT") {
                    for part in var_part.trim().split_whitespace() {
                        if part.starts_with('?') {
                            variables.push(Variable(part[1..].to_string()));
                        }
                    }
                }
            } else if line.starts_with("CONSTRUCT") {
                query_type = QueryType::Construct(vec![]); // TODO: テンプレート解析
            } else if line.starts_with("ASK") {
                query_type = QueryType::Ask;
            } else if line.starts_with("DESCRIBE") {
                query_type = QueryType::Describe(vec![]); // TODO: 変数/IRI解析
            }
        }

        Ok(SparqlQuery {
            query_type,
            variables,
            dataset: vec![],
            where_clause: GraphPattern::Bgp(vec![]), // TODO: WHERE句解析
            solution_modifier: SolutionModifier {
                group: None,
                having: None,
                order: None,
                limit: None,
                offset: None,
                distinct: false,
                reduced: false,
            },
            values: None,
            base_iri: None,
            prefixes,
        })
    }
}
