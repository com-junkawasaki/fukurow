// W3C SPARQL 1.1 Compliance Tests
// Based on the W3C SPARQL 1.1 Test Suite structure

use fukurow_sparql::parser::{SparqlParser, DefaultSparqlParser, QueryType};

#[test]
fn test_syntax_select_1() {
    // Basic SELECT query
    let query = r#"
    SELECT ?x ?y
    WHERE {
        ?x ?p ?y
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query).unwrap();

    assert_eq!(result.variables.len(), 2);
    match result.query_type {
        QueryType::Select => {},
        _ => panic!("Expected SELECT query"),
    }
}

#[test]
fn test_syntax_select_2() {
    // SELECT with DISTINCT
    let query = r#"
    SELECT DISTINCT ?x ?y
    WHERE {
        ?x ?p ?y
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query).unwrap();

    match result.query_type {
        QueryType::Select => {},
        _ => panic!("Expected SELECT query"),
    }
}

#[test]
fn test_syntax_select_3() {
    // SELECT with PREFIX
    let query = r#"
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    SELECT ?name
    WHERE {
        ?person foaf:name ?name
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query).unwrap();

    assert_eq!(result.variables.len(), 1);
    assert!(result.prefixes.contains_key("foaf"));
    assert_eq!(result.prefixes["foaf"].0, "http://xmlns.com/foaf/0.1/");
}

#[test]
fn test_syntax_construct_1() {
    // Basic CONSTRUCT query
    let query = r#"
    CONSTRUCT {
        ?x ?p ?y
    }
    WHERE {
        ?x ?p ?y
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query).unwrap();

    match result.query_type {
        QueryType::Construct(_) => {},
        _ => panic!("Expected CONSTRUCT query"),
    }
}

#[test]
fn test_syntax_ask_1() {
    // Basic ASK query
    let query = r#"
    ASK {
        ?x ?p ?y
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query).unwrap();

    match result.query_type {
        QueryType::Ask => {},
        _ => panic!("Expected ASK query"),
    }
}

#[test]
fn test_syntax_describe_1() {
    // Basic DESCRIBE query
    let query = r#"
    DESCRIBE ?x
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query).unwrap();

    match result.query_type {
        QueryType::Describe(_) => {},
        _ => panic!("Expected DESCRIBE query"),
    }
}

#[test]
fn test_multiple_prefixes() {
    let query = r#"
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
    SELECT ?name
    WHERE {
        ?person foaf:name ?name
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query).unwrap();

    assert_eq!(result.prefixes.len(), 3);
    assert!(result.prefixes.contains_key("foaf"));
    assert!(result.prefixes.contains_key("rdf"));
    assert!(result.prefixes.contains_key("rdfs"));
}
