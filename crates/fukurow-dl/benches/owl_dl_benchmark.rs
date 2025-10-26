use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fukurow_dl::{OwlDlReasoner, OwlDlOntology};
use fukurow_lite::{Class, Property, Individual, OwlIri, Axiom as LiteAxiom};
use fukurow_store::store::RdfStore;
use fukurow_store::provenance::{Provenance, GraphId};

fn create_test_dl_ontology(size: usize) -> OwlDlOntology {
    let mut store = RdfStore::new();
    let graph_id = GraphId::Named("bench".to_string());
    let provenance = Provenance::Sensor {
        source: "benchmark".to_string(),
        confidence: Some(1.0),
    };

    // Create a basic OWL DL ontology
    for i in 0..size {
        let class_iri = format!("http://example.org/Class{}", i);
        let prop_iri = format!("http://example.org/property{}", i);
        let individual_iri = format!("http://example.org/ind{}", i);

        // Class declaration
        store.insert(fukurow_core::model::Triple {
            subject: class_iri.clone(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            object: "http://www.w3.org/2002/07/owl#Class".to_string(),
        }, graph_id.clone(), provenance.clone());

        // Property declaration
        store.insert(fukurow_core::model::Triple {
            subject: prop_iri.clone(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            object: "http://www.w3.org/2002/07/owl#ObjectProperty".to_string(),
        }, graph_id.clone(), provenance.clone());

        // Individual declaration
        store.insert(fukurow_core::model::Triple {
            subject: individual_iri.clone(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            object: "http://www.w3.org/2002/07/owl#NamedIndividual".to_string(),
        }, graph_id.clone(), provenance.clone());

        // Class assertion
        store.insert(fukurow_core::model::Triple {
            subject: individual_iri.clone(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            object: class_iri.clone(),
        }, graph_id.clone(), provenance.clone());

        // Subclass relation (except for root)
        if i > 0 {
            let parent_class = format!("http://example.org/Class{}", i - 1);
            store.insert(fukurow_core::model::Triple {
                subject: class_iri.clone(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                object: parent_class,
            }, graph_id.clone(), provenance.clone());
        }
    }

    let reasoner = OwlDlReasoner::new();
    reasoner.load_ontology(&store).unwrap()
}

fn benchmark_dl_consistency_check(c: &mut Criterion) {
    let sizes = vec![5, 10, 20]; // Smaller sizes for OWL DL complexity

    for size in sizes {
        let ontology = create_test_dl_ontology(size);
        c.bench_function(&format!("owl_dl_consistency_{}_entities", size), |b| {
            b.iter(|| {
                let mut reasoner = OwlDlReasoner::new();
                let _result = reasoner.is_consistent(black_box(&ontology)).unwrap();
            });
        });
    }
}

fn benchmark_dl_loading(c: &mut Criterion) {
    let sizes = vec![5, 10, 20];

    for size in sizes {
        let mut store = RdfStore::new();
        let graph_id = GraphId::Named("bench".to_string());
        let provenance = Provenance::Sensor {
            source: "benchmark".to_string(),
            confidence: Some(1.0),
        };

        // Create test data
        for i in 0..size {
            let class_iri = format!("http://example.org/Class{}", i);
            store.insert(fukurow_core::model::Triple {
                subject: class_iri.clone(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            }, graph_id.clone(), provenance.clone());
        }

        c.bench_function(&format!("owl_dl_loading_{}_entities", size), |b| {
            b.iter(|| {
                let reasoner = OwlDlReasoner::new();
                let _ontology = reasoner.load_ontology(black_box(&store)).unwrap();
            });
        });
    }
}

fn benchmark_dl_to_lite_conversion(c: &mut Criterion) {
    let sizes = vec![5, 10, 20];

    for size in sizes {
        let ontology = create_test_dl_ontology(size);
        let reasoner = OwlDlReasoner::new();

        c.bench_function(&format!("owl_dl_to_lite_{}_entities", size), |b| {
            b.iter(|| {
                let _lite_ontology = reasoner.to_owl_lite(black_box(&ontology)).unwrap();
            });
        });
    }
}

criterion_group!(benches, benchmark_dl_consistency_check, benchmark_dl_loading, benchmark_dl_to_lite_conversion);
criterion_main!(benches);
