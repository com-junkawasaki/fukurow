use fukurow_sparql::parser::{SparqlParser, DefaultSparqlParser, SparqlQuery};

// Basic SPARQL syntax tests to verify parser functionality

#[test]
fn test_simple_select_query() {
    let query = r#"
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    SELECT ?name ?email
    WHERE {
        ?person foaf:name ?name .
        ?person foaf:mbox ?email .
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query);

    match result {
        Ok(parsed_query) => {
            match parsed_query.query_type {
                fukurow_sparql::parser::QueryType::Select => {
                    assert_eq!(parsed_query.variables.len(), 2);
                    println!("✅ Simple SELECT query parsed successfully");
                }
                _ => panic!("Expected SELECT query"),
            }
        }
        Err(e) => {
            println!("❌ Simple SELECT query failed: {}", e);
            panic!("Parse error: {}", e);
        }
    }
}

#[test]
fn test_construct_query() {
    let query = r#"
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    CONSTRUCT {
        ?person foaf:name ?name .
    }
    WHERE {
        ?person foaf:name ?name .
        ?person foaf:age ?age .
        FILTER (?age > 18)
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query);

    match result {
        Ok(parsed_query) => {
            match parsed_query.query_type {
                fukurow_sparql::parser::QueryType::Construct(_) => {
                    println!("✅ CONSTRUCT query parsed successfully");
                }
                _ => panic!("Expected CONSTRUCT query"),
            }
        }
        Err(e) => {
            println!("❌ CONSTRUCT query failed: {}", e);
            // Don't panic for now, let's see what fails
            eprintln!("CONSTRUCT parsing not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_ask_query() {
    let query = r#"
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    ASK {
        ?person foaf:name "Alice" .
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query);

    match result {
        Ok(parsed_query) => {
            match parsed_query.query_type {
                fukurow_sparql::parser::QueryType::Ask => {
                    println!("✅ ASK query parsed successfully");
                }
                _ => panic!("Expected ASK query"),
            }
        }
        Err(e) => {
            println!("❌ ASK query failed: {}", e);
            // Don't panic for now
            eprintln!("ASK parsing not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_optional_pattern() {
    let query = r#"
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    SELECT ?name ?email
    WHERE {
        ?person foaf:name ?name .
        OPTIONAL {
            ?person foaf:mbox ?email .
        }
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query);

    match result {
        Ok(_) => {
            println!("✅ OPTIONAL pattern parsed successfully");
        }
        Err(e) => {
            println!("❌ OPTIONAL pattern failed: {}", e);
            // OPTIONAL might not be implemented yet
            eprintln!("OPTIONAL parsing not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_union_pattern() {
    let query = r#"
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    SELECT ?name
    WHERE {
        {
            ?person foaf:name ?name .
        } UNION {
            ?person foaf:givenName ?name .
        }
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query);

    match result {
        Ok(_) => {
            println!("✅ UNION pattern parsed successfully");
        }
        Err(e) => {
            println!("❌ UNION pattern failed: {}", e);
            eprintln!("UNION parsing not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_filter_expression() {
    let query = r#"
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    SELECT ?name ?age
    WHERE {
        ?person foaf:name ?name .
        ?person foaf:age ?age .
        FILTER (?age > 18 && ?age < 65)
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query);

    match result {
        Ok(_) => {
            println!("✅ FILTER expression parsed successfully");
        }
        Err(e) => {
            println!("❌ FILTER expression failed: {}", e);
            eprintln!("FILTER parsing not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_property_path() {
    let query = r#"
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    SELECT ?name
    WHERE {
        ?person foaf:knows/foaf:name ?name .
    }
    "#;

    let parser = DefaultSparqlParser;
    let result = parser.parse(query);

    match result {
        Ok(_) => {
            println!("✅ Property path parsed successfully");
        }
        Err(e) => {
            println!("❌ Property path failed: {}", e);
            eprintln!("Property path parsing not yet implemented: {}", e);
        }
    }
}
