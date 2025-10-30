use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use fukurow_core::model::Triple;
use fukurow_store::store::RdfStore;
use fukurow_store::provenance::{Provenance, GraphId};
use fukurow_sparql::{SparqlQuery, QueryType, SparqlParser, SparqlEvaluator, DefaultSparqlEvaluator, DefaultSparqlOptimizer};

/// Generate test data for SPARQL benchmarks
fn generate_test_data(size: usize) -> RdfStore {
    let mut store = RdfStore::new();
    let graph_id = GraphId::Named("bench".to_string());
    let provenance = Provenance::Sensor {
        source: "benchmark".to_string(),
        confidence: Some(1.0),
    };

    // Create a social network-like dataset
    for i in 0..size {
        let person_uri = format!("http://example.org/person{}", i);
        let name = format!("\"Person {}\"", i);
        let age = i % 100 + 18; // Ages 18-117

        // Basic properties
        store.insert(Triple {
            subject: person_uri.clone(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            object: "http://example.org/Person".to_string(),
        }, graph_id.clone(), provenance.clone());

        store.insert(Triple {
            subject: person_uri.clone(),
            predicate: "http://example.org/name".to_string(),
            object: name,
        }, graph_id.clone(), provenance.clone());

        store.insert(Triple {
            subject: person_uri.clone(),
            predicate: "http://example.org/age".to_string(),
            object: format!("\"{}\"^^<http://www.w3.org/2001/XMLSchema#integer>", age),
        }, graph_id.clone(), provenance.clone());

        // Create relationships (friends, family)
        if i > 0 {
            // Friend relationship
            store.insert(Triple {
                subject: person_uri.clone(),
                predicate: "http://example.org/knows".to_string(),
                object: format!("http://example.org/person{}", i - 1),
            }, graph_id.clone(), provenance.clone());

            // Family relationships for some
            if i % 10 == 0 && i < size - 5 {
                store.insert(Triple {
                    subject: person_uri.clone(),
                    predicate: "http://example.org/hasChild".to_string(),
                    object: format!("http://example.org/person{}", i + 1),
                }, graph_id.clone(), provenance.clone());
            }
        }

        // Add interests
        for j in 0..3 {
            let interest = format!("http://example.org/interest{}", (i + j) % 50);
            store.insert(Triple {
                subject: person_uri.clone(),
                predicate: "http://example.org/interestedIn".to_string(),
                object: interest,
            }, graph_id.clone(), provenance.clone());
        }
    }

    store
}

/// Benchmark SPARQL parsing performance
fn benchmark_sparql_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparql_parsing");

    let queries = vec![
        ("simple_select", "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 100"),
        ("filter_query", "SELECT ?person ?name WHERE { ?person <http://example.org/name> ?name . ?person <http://example.org/age> ?age . FILTER(?age > 25) }"),
        ("join_query", "SELECT ?person1 ?person2 WHERE { ?person1 <http://example.org/knows> ?person2 . ?person2 <http://example.org/name> ?name }"),
        ("aggregate_query", "SELECT (COUNT(?person) AS ?count) WHERE { ?person <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> }"),
        ("complex_join", "SELECT ?person ?friend ?interest WHERE { ?person <http://example.org/knows> ?friend . ?person <http://example.org/interestedIn> ?interest . ?friend <http://example.org/interestedIn> ?interest }"),
    ];

    for (name, query) in queries {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            query,
            |b, query| {
                b.iter(|| {
                    let parser = SparqlParser::new();
                    let _parsed = parser.parse(black_box(query)).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark SPARQL query execution
fn benchmark_sparql_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparql_execution");

    // Create test datasets of different sizes
    let datasets = vec![
        ("small", 100),
        ("medium", 1000),
        ("large", 5000),
    ];

    for (size_name, data_size) in datasets {
        let store = generate_test_data(data_size);
        let evaluator = DefaultSparqlEvaluator::new();

        // Simple SELECT query
        let simple_query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 100";
        let parsed_simple = SparqlParser::new().parse(simple_query).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_simple_select", size_name)),
            &(&store, &parsed_simple),
            |b, (store, query)| {
                b.iter(|| {
                    let _results = evaluator.evaluate(black_box(query), black_box(store)).unwrap();
                });
            },
        );

        // Filter query
        let filter_query = "SELECT ?person ?name WHERE { ?person <http://example.org/name> ?name . ?person <http://example.org/age> ?age . FILTER(?age > 25) }";
        let parsed_filter = SparqlParser::new().parse(filter_query).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_filter_query", size_name)),
            &(&store, &parsed_filter),
            |b, (store, query)| {
                b.iter(|| {
                    let _results = evaluator.evaluate(black_box(query), black_box(store)).unwrap();
                });
            },
        );

        // Join query
        let join_query = "SELECT ?person1 ?person2 WHERE { ?person1 <http://example.org/knows> ?person2 . ?person2 <http://example.org/name> ?name }";
        let parsed_join = SparqlParser::new().parse(join_query).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_join_query", size_name)),
            &(&store, &parsed_join),
            |b, (store, query)| {
                b.iter(|| {
                    let _results = evaluator.evaluate(black_box(query), black_box(store)).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark SPARQL optimization
fn benchmark_sparql_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparql_optimization");

    let queries = vec![
        ("simple_select", "SELECT ?s ?p ?o WHERE { ?s ?p ?o }"),
        ("complex_filter", "SELECT ?person ?name ?age WHERE { ?person <http://example.org/name> ?name . ?person <http://example.org/age> ?age . ?person <http://example.org/knows> ?friend . FILTER(?age > 25 && ?age < 65) }"),
        ("nested_optional", "SELECT ?person ?name WHERE { ?person <http://example.org/name> ?name OPTIONAL { ?person <http://example.org/age> ?age } OPTIONAL { ?person <http://example.org/knows> ?friend } }"),
    ];

    for (name, query_str) in queries {
        let query = SparqlParser::new().parse(query_str).unwrap();
        let optimizer = DefaultSparqlOptimizer::default();

        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &query,
            |b, query| {
                b.iter(|| {
                    let _optimized = optimizer.optimize(black_box(query)).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark end-to-end SPARQL processing (parse + optimize + execute)
fn benchmark_end_to_end_sparql(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparql_end_to_end");

    let store = generate_test_data(2000);
    let evaluator = DefaultSparqlEvaluator::new();
    let optimizer = DefaultSparqlOptimizer::default();

    let queries = vec![
        ("basic_lookup", "SELECT ?person ?name WHERE { ?person <http://example.org/name> ?name }"),
        ("filtered_lookup", "SELECT ?person ?name ?age WHERE { ?person <http://example.org/name> ?name . ?person <http://example.org/age> ?age . FILTER(?age > 30) }"),
        ("relationship_query", "SELECT ?person1 ?person2 WHERE { ?person1 <http://example.org/knows> ?person2 }"),
        ("complex_relationship", "SELECT ?person ?friend ?interest WHERE { ?person <http://example.org/knows> ?friend . ?person <http://example.org/interestedIn> ?interest . ?friend <http://example.org/interestedIn> ?interest }"),
    ];

    for (name, query_str) in queries {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            query_str,
            |b, query_str| {
                b.iter(|| {
                    // Parse
                    let query = SparqlParser::new().parse(black_box(query_str)).unwrap();

                    // Optimize
                    let optimized_query = optimizer.optimize(&query).unwrap();

                    // Execute
                    let _results = evaluator.evaluate(black_box(&optimized_query), black_box(&store)).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark SPARQL result processing
fn benchmark_result_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparql_results");

    let store = generate_test_data(5000);
    let evaluator = DefaultSparqlEvaluator::new();

    let queries = vec![
        ("small_result_set", "SELECT ?person WHERE { ?person <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> } LIMIT 10"),
        ("medium_result_set", "SELECT ?person ?name WHERE { ?person <http://example.org/name> ?name } LIMIT 100"),
        ("large_result_set", "SELECT ?person ?name ?age WHERE { ?person <http://example.org/name> ?name . ?person <http://example.org/age> ?age } LIMIT 1000"),
    ];

    for (name, query_str) in queries {
        let query = SparqlParser::new().parse(query_str).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &query,
            |b, query| {
                b.iter(|| {
                    let results = evaluator.evaluate(black_box(query), black_box(&store)).unwrap();

                    // Process results (simulate real-world usage)
                    let mut count = 0;
                    let mut bindings_count = 0;

                    match results {
                        fukurow_sparql::QueryResult::Bindings(bindings) => {
                            for binding in bindings {
                                count += 1;
                                bindings_count += binding.len();
                            }
                        }
                        _ => {}
                    }

                    black_box((count, bindings_count));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_sparql_parsing,
    benchmark_sparql_execution,
    benchmark_sparql_optimization,
    benchmark_end_to_end_sparql,
    benchmark_result_processing
);
criterion_main!(benches);
