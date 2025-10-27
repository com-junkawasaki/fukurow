use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use fukurow_core::model::{Triple, JsonLdDocument};
use fukurow_core::store::GraphStore;
use fukurow_core::query::{GraphQuery, var, const_val};
use std::collections::HashMap;

/// Generate test triples for core benchmarks
fn generate_test_triples(count: usize) -> Vec<Triple> {
    let mut triples = Vec::with_capacity(count);

    for i in 0..count {
        triples.push(Triple {
            subject: format!("http://example.org/subject{}", i),
            predicate: "http://example.org/predicate".to_string(),
            object: format!("http://example.org/object{}", i % 100), // Reuse objects
        });
    }

    triples
}

/// Generate complex JSON-LD documents for testing
fn generate_jsonld_documents(count: usize) -> Vec<JsonLdDocument> {
    let mut documents = Vec::with_capacity(count);

    for i in 0..count {
        let mut data = HashMap::new();
        data.insert("@type".to_string(), serde_json::Value::String("http://example.org/TestClass".to_string()));
        data.insert("name".to_string(), serde_json::Value::String(format!("Document {}", i)));
        data.insert("value".to_string(), serde_json::Value::Number((i as i64).into()));

        // Add some nested structure
        let mut nested = serde_json::Map::new();
        nested.insert("innerProperty".to_string(), serde_json::Value::String(format!("Inner {}", i)));
        data.insert("nested".to_string(), serde_json::Value::Object(nested));

        documents.push(JsonLdDocument {
            context: serde_json::json!({
                "@base": "http://example.org/",
                "name": "http://example.org/name",
                "value": "http://example.org/value",
                "nested": "http://example.org/nested"
            }),
            graph: None,
            data,
        });
    }

    documents
}

/// Benchmark triple creation and basic operations
fn benchmark_triple_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("triple_operations");

    group.bench_function("triple_creation", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let triple = Triple {
                    subject: format!("http://example.org/subject{}", i),
                    predicate: "http://example.org/predicate".to_string(),
                    object: format!("http://example.org/object{}", i),
                };
                black_box(triple);
            }
        });
    });

    group.bench_function("triple_cloning", |b| {
        let triples: Vec<Triple> = (0..1000).map(|i| Triple {
            subject: format!("http://example.org/subject{}", i),
            predicate: "http://example.org/predicate".to_string(),
            object: format!("http://example.org/object{}", i),
        }).collect();

        b.iter(|| {
            let cloned: Vec<Triple> = triples.iter().map(|t| t.clone()).collect();
            black_box(cloned);
        });
    });

    group.bench_function("triple_hashing", |b| {
        let triples: Vec<Triple> = (0..1000).map(|i| Triple {
            subject: format!("http://example.org/subject{}", i),
            predicate: "http://example.org/predicate".to_string(),
            object: format!("http://example.org/object{}", i),
        }).collect();

        b.iter(|| {
            let mut hashes = Vec::with_capacity(triples.len());
            for triple in &triples {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                triple.hash(&mut hasher);
                hashes.push(hasher.finish());
            }
            black_box(hashes);
        });
    });

    group.finish();
}

/// Benchmark GraphStore operations
fn benchmark_graph_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_store");

    for size in [100, 1000, 10000].iter() {
        let triples = generate_test_triples(*size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("insert_{}_triples", size)),
            &triples,
            |b, triples| {
                b.iter(|| {
                    let mut store = GraphStore::new();
                    for triple in triples.iter() {
                        store.add_triple(triple.clone());
                    }
                    black_box(&store);
                });
            },
        );

        // Query benchmarks
        let mut store = GraphStore::new();
        for triple in triples.iter() {
            store.add_triple(triple.clone());
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("query_{}_triples", size)),
            &store,
            |b, store| {
                b.iter(|| {
                    let results = store.find_triples(None, None, None);
                    black_box(results);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("contains_{}_triples", size)),
            &store,
            |b, store| {
                b.iter(|| {
                    let mut found = 0;
                    for i in 0..100.min(*size) {
                        let triple = &triples[i];
                        // Check if triple exists by trying to find it
                        let results = store.find_triples(Some(&triple.subject), Some(&triple.predicate), Some(&triple.object));
                        if !results.is_empty() {
                            found += 1;
                        }
                    }
                    black_box(found);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark JSON-LD document operations
fn benchmark_jsonld_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonld_operations");

    for size in [10, 50, 100].iter() {
        let documents = generate_jsonld_documents(*size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("jsonld_serialize_{}_docs", size)),
            &documents,
            |b, documents| {
                b.iter(|| {
                    let mut serialized = Vec::with_capacity(documents.len());
                    for doc in documents.iter() {
                        let json = serde_json::to_string(doc).unwrap();
                        serialized.push(json);
                    }
                    black_box(serialized);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("jsonld_deserialize_{}_docs", size)),
            &documents,
            |b, documents| {
                b.iter(|| {
                    let mut deserialized = Vec::with_capacity(documents.len());
                    for doc in documents.iter() {
                        let json = serde_json::to_string(doc).unwrap();
                        let parsed: JsonLdDocument = serde_json::from_str(&json).unwrap();
                        deserialized.push(parsed);
                    }
                    black_box(deserialized);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark GraphQuery operations
fn benchmark_graph_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_queries");

    // Create a test store with known data
    let mut store = GraphStore::new();
    let test_triples = generate_test_triples(1000);
    for triple in &test_triples {
        store.add_triple(triple.clone());
    }

    let queries = vec![
        ("exact_match", GraphQuery::new().where_clause(
            const_val("http://example.org/subject0"),
            const_val("http://example.org/predicate"),
            const_val("http://example.org/object0")
        )),
        ("subject_pattern", GraphQuery::new().where_clause(
            const_val("http://example.org/subject0"),
            var("p"),
            var("o")
        )),
        ("predicate_pattern", GraphQuery::new().where_clause(
            var("s"),
            const_val("http://example.org/predicate"),
            var("o")
        )),
        ("object_pattern", GraphQuery::new().where_clause(
            var("s"),
            var("p"),
            const_val("http://example.org/object0")
        )),
        ("open_pattern", GraphQuery::new().where_clause(
            var("s"),
            var("p"),
            var("o")
        )),
    ];

    for (name, query) in queries {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(&store, &query),
            |b, (store, query)| {
                b.iter(|| {
                    let results = query.execute(store);
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark pattern count operations
fn benchmark_pattern_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_counting");

    for size in [100, 1000, 10000].iter() {
        let store = GraphStore::new();
        // Note: pattern_count method may not be implemented yet, so this is a placeholder
        // When implemented, this would test the pattern counting functionality

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("pattern_count_{}_triples", size)),
            size,
            |b, &size| {
                b.iter(|| {
                    // Simulate pattern counting logic
                    let triples = generate_test_triples(size);

                    // Count patterns manually (what pattern_count would do)
                    let mut subject_patterns = std::collections::HashSet::new();
                    let mut predicate_patterns = std::collections::HashSet::new();
                    let mut object_patterns = std::collections::HashSet::new();

                    for triple in &triples {
                        subject_patterns.insert(&triple.subject);
                        predicate_patterns.insert(&triple.predicate);
                        object_patterns.insert(&triple.object);
                    }

                    black_box((subject_patterns.len(), predicate_patterns.len(), object_patterns.len()));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns
fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    for size in [1000, 10000, 50000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("memory_{}_triples", size)),
            size,
            |b, &size| {
                b.iter(|| {
                    let triples = generate_test_triples(size);
                    let mut store = GraphStore::new();

                    // Insert all triples
                    for triple in &triples {
                        store.add_triple(triple.clone());
                    }

                    // Force memory allocation measurement
                    let all_triples = store.find_triples(None, None, None);
                    let count = all_triples.len();

                    black_box((store, count));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark JSON-LD conversion
fn benchmark_jsonld_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonld_conversion");

    for size in [100, 1000, 5000].iter() {
        let mut store = GraphStore::new();
        let test_triples = generate_test_triples(*size);
        for triple in &test_triples {
            store.add_triple(triple.clone());
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("jsonld_{}_triples", size)),
            &store,
            |b, store| {
                b.iter(|| {
                    let jsonld = store.to_jsonld().unwrap();
                    black_box(jsonld);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_triple_operations,
    benchmark_graph_store,
    benchmark_jsonld_operations,
    benchmark_graph_queries,
    benchmark_pattern_counting,
    benchmark_memory_usage,
    benchmark_jsonld_conversion
);
criterion_main!(benches);
