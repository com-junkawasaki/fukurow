use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fukurow_core::model::Triple;
use fukurow_store::store::RdfStore;
use fukurow_store::provenance::{Provenance, GraphId};
use fukurow_rdfs::{RdfsReasoner, vocabulary};

fn default_graph_id() -> GraphId {
    GraphId::Named("bench".to_string())
}

fn sensor_provenance() -> Provenance {
    Provenance::Sensor {
        source: "benchmark".to_string(),
        confidence: Some(1.0),
    }
}

fn create_test_ontology(size: usize) -> RdfStore {
    let mut store = RdfStore::new();

    // Create a class hierarchy: Root <- L1 <- L2 <- ... <- Leaf
    for i in 0..size {
        let class_i = format!("http://example.org/Class{}", i);

        if i > 0 {
            let parent_class = format!("http://example.org/Class{}", i - 1);
            // Class_i subclassOf Class_{i-1}
            store.insert(Triple {
                subject: class_i.clone(),
                predicate: vocabulary::rdfs_subclass_of().as_str().to_string(),
                object: parent_class,
            }, default_graph_id(), sensor_provenance());
        }

        // Add some instances for each class
        for j in 0..10 {
            let instance = format!("http://example.org/Instance{}{}", i, j);
            store.insert(Triple {
                subject: instance,
                predicate: vocabulary::rdf_type().as_str().to_string(),
                object: class_i.clone(),
            }, default_graph_id(), sensor_provenance());
        }

        // Add some properties with domain/range
        if i % 3 == 0 {
            let property = format!("http://example.org/Property{}", i);
            store.insert(Triple {
                subject: property.clone(),
                predicate: vocabulary::rdfs_domain().as_str().to_string(),
                object: class_i.clone(),
            }, default_graph_id(), sensor_provenance());

            let range_class = format!("http://example.org/RangeClass{}", i);
            store.insert(Triple {
                subject: property,
                predicate: vocabulary::rdfs_range().as_str().to_string(),
                object: range_class,
            }, default_graph_id(), sensor_provenance());
        }
    }

    store
}

fn benchmark_rdfs_inference(c: &mut Criterion) {
    let sizes = [100, 500, 1000, 2000];

    for &size in &sizes {
        let store = create_test_ontology(size);

        c.bench_function(&format!("rdfs_inference_{}_classes", size), |b| {
            b.iter(|| {
                let mut reasoner = RdfsReasoner::new();
                let _result = reasoner.compute_closure(black_box(&store)).unwrap();
            });
        });
    }
}

fn benchmark_large_ontology(c: &mut Criterion) {
    // Create a large ontology similar to real-world scenarios
    let store = create_test_ontology(1000); // ~10k triples

    c.bench_function("rdfs_inference_10k_triples", |b| {
        b.iter(|| {
            let mut reasoner = RdfsReasoner::new();
            let _result = reasoner.compute_closure(black_box(&store)).unwrap();
        });
    });
}

fn benchmark_class_hierarchy_only(c: &mut Criterion) {
    let mut store = RdfStore::new();

    // Create a deep class hierarchy (100 levels)
    for i in 1..101 {
        let child_class = format!("http://example.org/Class{}", i);
        let parent_class = format!("http://example.org/Class{}", i - 1);

        store.insert(Triple {
            subject: child_class,
            predicate: vocabulary::rdfs_subclass_of().as_str().to_string(),
            object: parent_class,
        }, default_graph_id(), sensor_provenance());
    }

    c.bench_function("rdfs_class_hierarchy_100_levels", |b| {
        b.iter(|| {
            let mut reasoner = RdfsReasoner::new();
            let _result = reasoner.compute_closure(black_box(&store)).unwrap();
        });
    });
}

criterion_group!(
    benches,
    benchmark_rdfs_inference,
    benchmark_large_ontology,
    benchmark_class_hierarchy_only
);
criterion_main!(benches);
