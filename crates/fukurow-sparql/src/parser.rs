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
    PrefixedName(String, String), // (prefix, local_name)
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
    fn parse(&self, query: &str) -> Result<SparqlQuery, crate::SparqlError>;
    fn parse_query(&self, query: &str) -> Result<SparqlQuery, crate::SparqlError>;
}

/// Default SPARQL Parser with winnow
pub struct DefaultSparqlParser;

impl DefaultSparqlParser {
    /// Parse IRI (e.g., <http://example.org>)
    fn parse_iri(input: &mut &str) -> winnow::ModalResult<Iri> {
        winnow::combinator::delimited(
            '<',
            winnow::token::take_while(1.., |c: char| c != '>'),
            '>'
        ).parse_next(input)
        .map(|s: &str| Iri(s.to_string()))
    }

    /// Parse variable (e.g., ?x)
    fn parse_variable(input: &mut &str) -> winnow::ModalResult<Variable> {
        winnow::combinator::preceded(
            '?',
            winnow::token::take_while(1.., |c: char| c.is_alphanumeric() || c == '_')
        ).parse_next(input)
        .map(|s: &str| Variable(s.to_string()))
    }

    /// Parse prefixed name (e.g., rdf:type)
    fn parse_prefixed_name(input: &mut &str) -> winnow::ModalResult<Term> {
        let prefix = winnow::token::take_while(1.., |c: char| c.is_alphanumeric() || c == '_');
        let local = winnow::combinator::preceded(
            ':',
            winnow::token::take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-')
        );

        (prefix, local).parse_next(input)
        .map(|(p, l): (&str, &str)| Term::PrefixedName(p.to_string(), l.to_string()))
    }

    /// Parse term (IRI, variable, literal, prefixed name)
    fn parse_term(input: &mut &str) -> winnow::ModalResult<Term> {
        winnow::combinator::alt((
            Self::parse_variable.map(Term::Variable),
            Self::parse_iri.map(Term::Iri),
            Self::parse_prefixed_name,
        )).parse_next(input)
    }

    /// Parse triple pattern
    fn parse_triple_pattern(input: &mut &str) -> winnow::ModalResult<TriplePattern> {
        let subject = Self::parse_term;
        let predicate = Self::parse_term;
        let object = Self::parse_term;

        (subject, predicate, object).parse_next(input)
        .map(|(s, p, o)| TriplePattern { subject: s, predicate: p, object: o })
    }

    /// Parse SELECT clause
    fn parse_select_clause(input: &mut &str) -> winnow::ModalResult<(bool, Vec<Variable>)> {
        let distinct = winnow::combinator::opt("DISTINCT").map(|d| d.is_some());
        let star_or_vars = winnow::combinator::alt((
            "*".map(|_| vec![]), // SELECT * - variables will be determined later
            winnow::combinator::repeat(1.., Self::parse_variable)
        ));

        winnow::combinator::preceded("SELECT", (distinct, star_or_vars)).parse_next(input)
    }

    /// Parse WHERE clause
    fn parse_where_clause(input: &mut &str) -> winnow::ModalResult<GraphPattern> {
        winnow::combinator::preceded(
            "WHERE",
            winnow::combinator::delimited(
                '{',
                Self::parse_group_graph_pattern,
                '}'
            )
        ).parse_next(input)
    }

    /// Parse group graph pattern (simplified - just BGP for now)
    fn parse_group_graph_pattern(input: &mut &str) -> winnow::ModalResult<GraphPattern> {
        let mut triples = winnow::combinator::repeat(
            0..,
            winnow::combinator::terminated(
                Self::parse_triple_pattern,
                winnow::combinator::opt('.')
            )
        );

        triples.parse_next(input).map(|ts| GraphPattern::Bgp(ts))
    }

    /// Parse PREFIX declaration
    fn parse_prefix_declaration(input: &mut &str) -> winnow::ModalResult<(String, Iri)> {
        let prefix = winnow::token::take_while(1.., |c: char| c.is_alphanumeric() || c == '_');
        let iri = winnow::combinator::delimited(
            '<',
            winnow::token::take_while(1.., |c: char| c != '>'),
            '>'
        );

        winnow::combinator::preceded(
            "PREFIX",
            (prefix, ':', iri)
        ).parse_next(input)
        .map(|(p, _, i): (&str, char, &str)| (p.to_string(), Iri(i.to_string())))
    }
}

impl SparqlParser for DefaultSparqlParser {
    fn parse(&self, query: &str) -> Result<SparqlQuery, crate::SparqlError> {
        // Simple line-based parsing for now
        let mut prefixes = HashMap::new();
        let mut variables = Vec::new();
        let mut in_where = false;
        let mut triples = Vec::new();

        for line in query.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("PREFIX") {
                // Parse PREFIX declaration
                if let Some(prefix_def) = line.strip_prefix("PREFIX") {
                    let parts: Vec<&str> = prefix_def.trim().split_whitespace().collect();
                    if parts.len() >= 2 {
                        let prefix = parts[0].trim_end_matches(':');
                        let iri_str = parts[1].trim_matches('<').trim_matches('>');
                        prefixes.insert(prefix.to_string(), Iri(iri_str.to_string()));
                    }
                }
            } else if line.starts_with("SELECT") {
                // Parse SELECT variables
                if let Some(var_part) = line.strip_prefix("SELECT") {
                    let var_part = var_part.trim();
                    if var_part.starts_with("DISTINCT") {
                        // TODO: Handle DISTINCT
                        continue;
                    }
                    if var_part == "*" {
                        // SELECT * - no specific variables
                        continue;
                    }
                    for part in var_part.split_whitespace() {
                        if part.starts_with('?') {
                            variables.push(Variable(part[1..].to_string()));
                        }
                    }
                }
            } else if line.starts_with("WHERE") {
                in_where = true;
            } else if in_where && line.contains('.') {
                // Parse triple pattern (very simple)
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let subject = if parts[0].starts_with('?') {
                        Term::Variable(Variable(parts[0][1..].to_string()))
                    } else if parts[0].starts_with('<') {
                        Term::Iri(Iri(parts[0].trim_matches('<').trim_matches('>').to_string()))
                    } else {
                        continue; // Skip complex patterns for now
                    };

                    let predicate = if parts[1].starts_with('<') {
                        Term::Iri(Iri(parts[1].trim_matches('<').trim_matches('>').to_string()))
                    } else if parts[1].contains(':') {
                        let colon_parts: Vec<&str> = parts[1].split(':').collect();
                        if colon_parts.len() == 2 {
                            Term::PrefixedName(colon_parts[0].to_string(), colon_parts[1].to_string())
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    };

                    let object = if parts[2].starts_with('?') {
                        Term::Variable(Variable(parts[2][1..].to_string()))
                    } else if parts[2].starts_with('<') {
                        Term::Iri(Iri(parts[2].trim_matches('<').trim_matches('>').to_string()))
                    } else if parts[2].contains(':') {
                        let colon_parts: Vec<&str> = parts[2].split(':').collect();
                        if colon_parts.len() == 2 {
                            Term::PrefixedName(colon_parts[0].to_string(), colon_parts[1].to_string())
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    };

                    triples.push(TriplePattern {
                        subject,
                        predicate,
                        object,
                    });
                }
            }
        }

        Ok(SparqlQuery {
            query_type: QueryType::Select,
            variables,
            dataset: vec![],
            where_clause: GraphPattern::Bgp(triples),
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


    fn parse_query(&self, query: &str) -> Result<SparqlQuery, crate::SparqlError> {
        self.parse(query)
    }
}
