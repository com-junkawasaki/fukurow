//! OWLオントロジーローダー

use crate::model::{Ontology, Class, Property, Individual, Axiom, OwlIri};
use crate::OwlError;
use fukurow_store::store::RdfStore;
use std::collections::HashMap;

/// Ontology loader trait
pub trait OntologyLoader {
    fn load_from_store(&self, store: &RdfStore) -> Result<Ontology, OwlError>;
}

/// Default OWL ontology loader
pub struct DefaultOntologyLoader;

impl OntologyLoader for DefaultOntologyLoader {
    fn load_from_store(&self, store: &RdfStore) -> Result<Ontology, OwlError> {
        let mut ontology = Ontology::new();

        // OWL/RDF vocabulary IRIs
        let rdf_type = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
        let rdfs_subclass_of = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
        let rdfs_domain = "http://www.w3.org/2000/01/rdf-schema#domain";
        let rdfs_range = "http://www.w3.org/2000/01/rdf-schema#range";
        let owl_class = "http://www.w3.org/2002/07/owl#Class";
        let owl_object_property = "http://www.w3.org/2002/07/owl#ObjectProperty";
        let owl_datatype_property = "http://www.w3.org/2002/07/owl#DatatypeProperty";
        let owl_named_individual = "http://www.w3.org/2002/07/owl#NamedIndividual";
        let owl_subclass_of = "http://www.w3.org/2002/07/owl#subClassOf";
        let owl_equivalent_class = "http://www.w3.org/2002/07/owl#equivalentClass";
        let owl_disjoint_with = "http://www.w3.org/2002/07/owl#disjointWith";
        let owl_sub_property_of = "http://www.w3.org/2002/07/owl#subPropertyOf";
        let owl_equivalent_property = "http://www.w3.org/2002/07/owl#equivalentProperty";
        let owl_functional_property = "http://www.w3.org/2002/07/owl#FunctionalProperty";
        let owl_inverse_functional_property = "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
        let owl_transitive_property = "http://www.w3.org/2002/07/owl#TransitiveProperty";
        let owl_symmetric_property = "http://www.w3.org/2002/07/owl#SymmetricProperty";

        // Process all triples to extract OWL axioms
        for stored_triple in store.all_triples().values().flatten() {
            let triple = &stored_triple.triple;

            // rdf:type declarations
            if triple.predicate == rdf_type {
                match triple.object.as_str() {
                    x if x == owl_class => {
                        let class = Class::Named(OwlIri::new(triple.subject.clone()));
                        ontology.classes.insert(class);
                    }
                    x if x == owl_object_property => {
                        let prop = Property::Object(OwlIri::new(triple.subject.clone()));
                        ontology.properties.insert(prop);
                    }
                    x if x == owl_datatype_property => {
                        let prop = Property::Data(OwlIri::new(triple.subject.clone()));
                        ontology.properties.insert(prop);
                    }
                    x if x == owl_named_individual => {
                        let individual = Individual(OwlIri::new(triple.subject.clone()));
                        ontology.individuals.insert(individual);
                    }
                    x if x == owl_functional_property => {
                        if let Some(prop) = self.find_property_by_iri(&ontology, &triple.subject) {
                            ontology.add_axiom(Axiom::FunctionalProperty(prop));
                        }
                    }
                    x if x == owl_inverse_functional_property => {
                        if let Some(prop) = self.find_property_by_iri(&ontology, &triple.subject) {
                            ontology.add_axiom(Axiom::InverseFunctionalProperty(prop));
                        }
                    }
                    x if x == owl_transitive_property => {
                        if let Some(prop) = self.find_property_by_iri(&ontology, &triple.subject) {
                            ontology.add_axiom(Axiom::TransitiveProperty(prop));
                        }
                    }
                    x if x == owl_symmetric_property => {
                        if let Some(prop) = self.find_property_by_iri(&ontology, &triple.subject) {
                            ontology.add_axiom(Axiom::SymmetricProperty(prop));
                        }
                    }
                    _ => {} // Other types are handled below
                }
            }

            // rdfs:subClassOf
            else if triple.predicate == rdfs_subclass_of {
                let c1 = Class::Named(OwlIri::new(triple.subject.clone()));
                let c2 = Class::Named(OwlIri::new(triple.object.clone()));
                ontology.add_axiom(Axiom::SubClassOf(c1, c2));
            }

            // owl:subClassOf
            else if triple.predicate == owl_subclass_of {
                let c1 = Class::Named(OwlIri::new(triple.subject.clone()));
                let c2 = Class::Named(OwlIri::new(triple.object.clone()));
                ontology.add_axiom(Axiom::SubClassOf(c1, c2));
            }

            // owl:equivalentClass
            else if triple.predicate == owl_equivalent_class {
                // For simplicity, treat as SubClassOf in both directions
                let c1 = Class::Named(OwlIri::new(triple.subject.clone()));
                let c2 = Class::Named(OwlIri::new(triple.object.clone()));
                ontology.add_axiom(Axiom::SubClassOf(c1.clone(), c2.clone()));
                ontology.add_axiom(Axiom::SubClassOf(c2, c1));
            }

            // rdfs:domain (for object properties)
            else if triple.predicate == rdfs_domain {
                if let Some(prop) = self.find_property_by_iri(&ontology, &triple.subject) {
                    if let Property::Object(_) = prop {
                        let class = Class::Named(OwlIri::new(triple.object.clone()));
                        ontology.add_axiom(Axiom::ObjectPropertyDomain(prop, class));
                    }
                }
            }

            // rdfs:range (for object properties)
            else if triple.predicate == rdfs_range {
                if let Some(prop) = self.find_property_by_iri(&ontology, &triple.subject) {
                    if let Property::Object(_) = prop {
                        let class = Class::Named(OwlIri::new(triple.object.clone()));
                        ontology.add_axiom(Axiom::ObjectPropertyRange(prop, class));
                    }
                }
            }

            // Class assertions (rdf:type with class IRI)
            else if triple.predicate == rdf_type {
                // Check if object is a class IRI (not a built-in OWL class)
                if !triple.object.starts_with("http://www.w3.org/2002/07/owl#") {
                    let class = Class::Named(OwlIri::new(triple.object.clone()));
                    let individual = Individual(OwlIri::new(triple.subject.clone()));
                    ontology.add_axiom(Axiom::ClassAssertion(class, individual));
                }
            }

            // Property assertions (object properties)
            else {
                // Check if predicate is an object property
                if let Some(prop) = self.find_property_by_iri(&ontology, &triple.predicate) {
                    if let Property::Object(_) = prop {
                        let i1 = Individual(OwlIri::new(triple.subject.clone()));
                        let i2 = Individual(OwlIri::new(triple.object.clone()));
                        ontology.add_axiom(Axiom::ObjectPropertyAssertion(prop, i1, i2));
                    }
                }
            }
        }

        Ok(ontology)
    }
}

impl DefaultOntologyLoader {
    fn find_property_by_iri(&self, ontology: &Ontology, iri: &str) -> Option<Property> {
        for prop in &ontology.properties {
            match prop {
                Property::Object(p_iri) | Property::Data(p_iri) => {
                    if p_iri.0 == iri {
                        return Some(prop.clone());
                    }
                }
            }
        }
        None
    }
}
