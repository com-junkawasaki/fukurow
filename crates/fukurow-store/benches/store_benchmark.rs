use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use fukurow_core::model::Triple;
use fukurow_store::store::RdfStore;
use fukurow_store::provenance::{Provenance, GraphId};
use std::collections::HashMap;

/// Generate a large set of test triples
fn generate_test_triples(count: usize) -> Vec<Triple> {
    let mut triples = Vec::with_capacity(count);

    for i in 0..count {
        // Create subjects in the format: http://example.org/subject_{i}
        let subject = format!("http://example.org/subject_{}", i);

        // Create multiple triples per subject
        triples.push(Triple {
            subject: subject.clone(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            object: format!("http://example.org/Class{}", i % 100), // 100 different classes
        });

        triples.push(Triple {
            subject: subject.clone(),
            predicate: "http://example.org/property1".to_string(),
            object: format!("http://example.org/value{}", i % 1000), // 1000 different values
        });

        triples.push(Triple {
            subject: subject.clone(),
            predicate: "http://example.org/property2".to_string(),
            object: format!("\"Literal value {}\"", i),
        });

        // Add some cross-references
        if i > 0 {
            triples.push(Triple {
                subject: subject.clone(),
                predicate: "http://example.org/refersTo".to_string(),
                object: format!("http://example.org/subject_{}", i - 1),
            });
        }
    }

    triples
}

/// Benchmark RDF store insertion performance
fn benchmark_store_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("rdf_store_insertion");

    for size in [100, 1000, 10000].iter() {
        let triples = generate_test_triples(*size);
        let graph_id = GraphId::Named("bench".to_string());
        let provenance = Provenance::Sensor {
            source: "benchmark".to_string(),
            confidence: Some(1.0),
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}k_triples", size / 1000)),
            &triples,
            |b, triples| {
                b.iter(|| {
                    let mut store = RdfStore::new();
                    for triple in triples.iter() {
                        store.insert(triple.clone(), graph_id.clone(), provenance.clone());
                    }
                    black_box(&store);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark RDF store query performance
fn benchmark_store_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("rdf_store_queries");

    // Setup: Create a store with 10k triples
    let triples = generate_test_triples(10000);
    let mut store = RdfStore::new();
    let graph_id = GraphId::Named("bench".to_string());
    let provenance = Provenance::Sensor {
        source: "benchmark".to_string(),
        confidence: Some(1.0),
    };

    for triple in triples.iter() {
        store.insert(triple.clone(), graph_id.clone(), provenance.clone());
    }

    // Benchmark different query types
    group.bench_function("find_by_subject", |b| {
        b.iter(|| {
            let results = store.find_by_subject(black_box("http://example.org/subject_5000"));
            black_box(results);
        });
    });

    group.bench_function("find_by_predicate", |b| {
        b.iter(|| {
            let results = store.find_by_predicate(black_box("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"));
            black_box(results);
        });
    });

    group.bench_function("find_by_object", |b| {
        b.iter(|| {
            let results = store.find_by_object(black_box("http://example.org/Class50"));
            black_box(results);
        });
    });

    group.bench_function("find_by_pattern_subject_predicate", |b| {
        b.iter(|| {
            let results = store.find_by_pattern(
                Some(black_box("http://example.org/subject_5000")),
                Some(black_box("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")),
                None,
            );
            black_box(results);
        });
    });

    group.finish();
}

/// Benchmark memory usage and store size
fn benchmark_store_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("rdf_store_memory");

    for size in [1000, 10000, 50000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}k_triples_memory", size / 1000)),
            size,
            |b, &size| {
                b.iter(|| {
                    let triples = generate_test_triples(size);
                    let mut store = RdfStore::new();
                    let graph_id = GraphId::Named("bench".to_string());
                    let provenance = Provenance::Sensor {
                        source: "benchmark".to_string(),
                        confidence: Some(1.0),
                    };

                    for triple in triples.iter() {
                        store.insert(triple.clone(), graph_id.clone(), provenance.clone());
                    }

                    // Force allocation measurement
                    black_box(store.all_triples());
                });
            },
        );
    }
    group.finish();
}

/// Benchmark provenance tracking performance
fn benchmark_provenance_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("provenance_operations");

    // Setup: Create a store with provenance data
    let triples = generate_test_triples(5000);
    let mut store = RdfStore::new();
    let graph_id = GraphId::Named("bench".to_string());

    // Create different types of provenance
    let provenances = vec![
        Provenance::Sensor {
            source: "sensor1".to_string(),
            confidence: Some(0.9),
        },
        Provenance::Inferred {
            rule: "test_rule".to_string(),
            reasoning_level: "rdfs".to_string(),
            evidence: vec!["evidence1".to_string()],
        },
        Provenance::Imported {
            source_uri: "http://example.org/data.ttl".to_string(),
            import_timestamp: None,
        },
    ];

    for (i, triple) in triples.iter().enumerate() {
        let provenance = provenances[i % provenances.len()].clone();
        store.insert(triple.clone(), graph_id.clone(), provenance);
    }

    group.bench_function("get_provenance_info", |b| {
        b.iter(|| {
            let info = store.get_provenance_info();
            black_box(info);
        });
    });

    group.bench_function("filter_by_confidence", |b| {
        b.iter(|| {
            let high_confidence = store.filter_by_confidence(0.8);
            black_box(high_confidence);
        });
    });

    group.finish();
}

/// Benchmark concurrent access patterns
fn benchmark_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");

    // Setup: Create a large store
    let triples = generate_test_triples(10000);
    let mut store = RdfStore::new();
    let graph_id = GraphId::Named("bench".to_string());
    let provenance = Provenance::Sensor {
        source: "benchmark".to_string(),
        confidence: Some(1.0),
    };

    for triple in triples.iter() {
        store.insert(triple.clone(), graph_id.clone(), provenance.clone());
    }

    group.bench_function("bulk_read_operations", |b| {
        b.iter(|| {
            // Simulate bulk read operations
            let all_triples = store.all_triples();
            let subject_count = all_triples.keys().len();
            let total_triples: usize = all_triples.values().map(|v| v.len()).sum();
            black_box((subject_count, total_triples));
        });
    });

    group.bench_function("index_lookup_performance", |b| {
        b.iter(|| {
            // Test index lookup performance
            for i in (0..1000).step_by(10) {
                let subject = format!("http://example.org/subject_{}", i);
                let results = store.find_by_subject(&subject);
                black_box(results);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_store_insertion,
    benchmark_store_queries,
    benchmark_store_memory,
    benchmark_provenance_operations,
    benchmark_concurrent_access
);
criterion_main!(benches);
