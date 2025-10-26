use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fukurow_lite::{OwlLiteReasoner, Ontology};
use fukurow_core::model::Triple;
use fukurow_store::store::RdfStore;
use fukurow_store::provenance::{Provenance, GraphId};

fn create_test_ontology(size: usize) -> Ontology {
    let mut store = RdfStore::new();
    let graph_id = GraphId::Named("bench".to_string());
    let provenance = Provenance::Sensor {
        source: "benchmark".to_string(),
        confidence: Some(1.0),
    };

    // Create a class hierarchy: Root <- L1 <- L2 <- ... <- Leaf
    for i in 0..size {
        let class_iri = format!("http://example.org/Class{}", i);

        // Class declaration
        store.insert(Triple {
            subject: class_iri.clone(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            object: "http://www.w3.org/2002/07/owl#Class".to_string(),
        }, graph_id.clone(), provenance.clone());

        // Subclass relation (except for root)
        if i > 0 {
            let parent_class = format!("http://example.org/Class{}", i - 1);
            store.insert(Triple {
                subject: class_iri.clone(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                object: parent_class,
            }, graph_id.clone(), provenance.clone());
        }

        // Add some individuals for each class
        for j in 0..5 {
            let individual_iri = format!("http://example.org/Individual{}{}", i, j);

            // Individual declaration
            store.insert(Triple {
                subject: individual_iri.clone(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
            }, graph_id.clone(), provenance.clone());

            // Class assertion
            store.insert(Triple {
                subject: individual_iri,
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: class_iri.clone(),
            }, graph_id.clone(), provenance.clone());
        }
    }

    let reasoner = OwlLiteReasoner::new();
    reasoner.load_ontology(&store).unwrap()
}

fn benchmark_consistency_check(c: &mut Criterion) {
    let sizes = vec![10, 50, 100];

    for size in sizes {
        let ontology = create_test_ontology(size);
        c.bench_function(&format!("owl_lite_consistency_{}_classes", size), |b| {
            b.iter(|| {
                let mut reasoner = OwlLiteReasoner::new();
                let _result = reasoner.is_consistent(black_box(&ontology)).unwrap();
            });
        });
    }
}

fn benchmark_class_hierarchy(c: &mut Criterion) {
    let sizes = vec![10, 50, 100];

    for size in sizes {
        let ontology = create_test_ontology(size);
        c.bench_function(&format!("owl_lite_hierarchy_{}_classes", size), |b| {
            b.iter(|| {
                let mut reasoner = OwlLiteReasoner::new();
                let _hierarchy = reasoner.compute_class_hierarchy(black_box(&ontology)).unwrap();
            });
        });
    }
}

criterion_group!(benches, benchmark_consistency_check, benchmark_class_hierarchy);
criterion_main!(benches);
