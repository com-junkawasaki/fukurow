//! OWL DLオントロジーローダー

use crate::model::{OwlDlOntology, ClassExpression, PropertyExpression, Axiom};
use fukurow_lite::model::OwlIri;
use crate::OwlDlError;
use fukurow_store::store::RdfStore;
use fukurow_lite::loader::{OntologyLoader, DefaultOntologyLoader};
use fukurow_lite::OwlLiteReasoner;
use std::collections::HashMap;

/// OWL DL ontology loader trait
pub trait OwlDlOntologyLoader {
    fn load_from_store(&self, store: &RdfStore) -> Result<OwlDlOntology, OwlDlError>;
}

/// Default OWL DL ontology loader
pub struct DefaultOwlDlOntologyLoader {
    lite_loader: DefaultOntologyLoader,
    lite_reasoner: OwlLiteReasoner,
}

impl DefaultOwlDlOntologyLoader {
    pub fn new() -> Self {
        Self {
            lite_loader: DefaultOntologyLoader,
            lite_reasoner: OwlLiteReasoner::new(),
        }
    }
}

impl OwlDlOntologyLoader for DefaultOwlDlOntologyLoader {
    fn load_from_store(&self, store: &RdfStore) -> Result<OwlDlOntology, OwlDlError> {
        // First load as OWL Lite
        let lite_ontology = self.lite_reasoner.load_ontology(store)
            .map_err(|e| OwlDlError::LoaderError(format!("Failed to load as OWL Lite: {}", e)))?;

        // Convert to OWL DL ontology
        let mut dl_ontology = OwlDlOntology::new();
        dl_ontology.iri = lite_ontology.iri;
        dl_ontology.classes = lite_ontology.classes;
        dl_ontology.properties = lite_ontology.properties;
        dl_ontology.individuals = lite_ontology.individuals;

        // Convert axioms
        for axiom in lite_ontology.axioms {
            dl_ontology.add_axiom(Axiom::OwlLite(axiom));
        }

        // Load additional OWL DL constructs
        self.load_dl_constructs(store, &mut dl_ontology)?;

        Ok(dl_ontology)
    }
}

impl DefaultOwlDlOntologyLoader {
    /// Load OWL DL specific constructs from RDF store
    fn load_dl_constructs(&self, store: &RdfStore, ontology: &mut OwlDlOntology) -> Result<(), OwlDlError> {
        // OWL DL vocabulary IRIs
        let owl_intersection_of = "http://www.w3.org/2002/07/owl#intersectionOf";
        let owl_union_of = "http://www.w3.org/2002/07/owl#unionOf";
        let owl_complement_of = "http://www.w3.org/2002/07/owl#complementOf";
        let owl_one_of = "http://www.w3.org/2002/07/owl#oneOf";
        let owl_some_values_from = "http://www.w3.org/2002/07/owl#someValuesFrom";
        let owl_all_values_from = "http://www.w3.org/2002/07/owl#allValuesFrom";
        let owl_has_value = "http://www.w3.org/2002/07/owl#hasValue";
        let owl_min_cardinality = "http://www.w3.org/2002/07/owl#minCardinality";
        let owl_max_cardinality = "http://www.w3.org/2002/07/owl#maxCardinality";
        let owl_exact_cardinality = "http://www.w3.org/2002/07/owl#exactCardinality";
        let owl_inverse_of = "http://www.w3.org/2002/07/owl#inverseOf";
        let owl_same_as = "http://www.w3.org/2002/07/owl#sameAs";
        let owl_different_from = "http://www.w3.org/2002/07/owl#differentFrom";

        // Maps to track complex constructs
        let mut intersection_classes = HashMap::new();
        let mut union_classes = HashMap::new();
        let mut complement_classes = HashMap::new();
        let mut enumeration_classes = HashMap::new();
        let mut restriction_classes = HashMap::new();
        let mut inverse_properties = HashMap::new();

        // Process all triples to extract OWL DL constructs
        for stored_triple in store.all_triples().values().flatten() {
            let triple = &stored_triple.triple;

            // intersectionOf
            if triple.predicate == owl_intersection_of {
                intersection_classes.insert(triple.subject.clone(), triple.object.clone());
            }

            // unionOf
            else if triple.predicate == owl_union_of {
                union_classes.insert(triple.subject.clone(), triple.object.clone());
            }

            // complementOf
            else if triple.predicate == owl_complement_of {
                complement_classes.insert(triple.subject.clone(), triple.object.clone());
            }

            // oneOf
            else if triple.predicate == owl_one_of {
                enumeration_classes.insert(triple.subject.clone(), triple.object.clone());
            }

            // someValuesFrom
            else if triple.predicate == owl_some_values_from {
                restriction_classes.entry(triple.subject.clone())
                    .or_insert_with(HashMap::new)
                    .insert("someValuesFrom".to_string(), triple.object.clone());
            }

            // allValuesFrom
            else if triple.predicate == owl_all_values_from {
                restriction_classes.entry(triple.subject.clone())
                    .or_insert_with(HashMap::new)
                    .insert("allValuesFrom".to_string(), triple.object.clone());
            }

            // hasValue
            else if triple.predicate == owl_has_value {
                restriction_classes.entry(triple.subject.clone())
                    .or_insert_with(HashMap::new)
                    .insert("hasValue".to_string(), triple.object.clone());
            }

            // minCardinality
            else if triple.predicate == owl_min_cardinality {
                restriction_classes.entry(triple.subject.clone())
                    .or_insert_with(HashMap::new)
                    .insert("minCardinality".to_string(), triple.object.clone());
            }

            // maxCardinality
            else if triple.predicate == owl_max_cardinality {
                restriction_classes.entry(triple.subject.clone())
                    .or_insert_with(HashMap::new)
                    .insert("maxCardinality".to_string(), triple.object.clone());
            }

            // exactCardinality
            else if triple.predicate == owl_exact_cardinality {
                restriction_classes.entry(triple.subject.clone())
                    .or_insert_with(HashMap::new)
                    .insert("exactCardinality".to_string(), triple.object.clone());
            }

            // inverseOf
            else if triple.predicate == owl_inverse_of {
                inverse_properties.insert(triple.subject.clone(), triple.object.clone());
            }
        }

        // Process intersection classes
        for (class_iri, list_iri) in intersection_classes {
            if let Some(expressions) = self.parse_rdf_list(store, &list_iri) {
                let class_expr = ClassExpression::IntersectionOf(
                    expressions.into_iter().map(|iri| self.iri_to_class_expression(&iri)).collect()
                );
                ontology.add_class_expression(&class_expr);
                // TODO: Add axiom that defines this class
            }
        }

        // Process union classes
        for (class_iri, list_iri) in union_classes {
            if let Some(expressions) = self.parse_rdf_list(store, &list_iri) {
                let class_expr = ClassExpression::UnionOf(
                    expressions.into_iter().map(|iri| self.iri_to_class_expression(&iri)).collect()
                );
                ontology.add_class_expression(&class_expr);
            }
        }

        // Process complement classes
        for (class_iri, complement_iri) in complement_classes {
            let complement_expr = self.iri_to_class_expression(&complement_iri);
            let class_expr = ClassExpression::ComplementOf(Box::new(complement_expr));
            ontology.add_class_expression(&class_expr);
        }

        // Process enumeration classes
        for (class_iri, list_iri) in enumeration_classes {
            if let Some(individuals) = self.parse_individual_list(store, &list_iri) {
                let class_expr = ClassExpression::OneOf(individuals);
                ontology.add_class_expression(&class_expr);
            }
        }

        // Process restrictions
        for (restriction_iri, properties) in restriction_classes {
            // Find the property this restriction is on
            let on_property = self.find_restriction_property(store, &restriction_iri)?;

            // Create appropriate restriction expression
            if let Some(some_values) = properties.get("someValuesFrom") {
                let class_expr = self.iri_to_class_expression(some_values);
                let restriction = ClassExpression::SomeValuesFrom {
                    property: on_property.clone(),
                    class: Box::new(class_expr),
                };
                ontology.add_class_expression(&restriction);
            }

            if let Some(all_values) = properties.get("allValuesFrom") {
                let class_expr = self.iri_to_class_expression(all_values);
                let restriction = ClassExpression::AllValuesFrom {
                    property: on_property.clone(),
                    class: Box::new(class_expr),
                };
                ontology.add_class_expression(&restriction);
            }

            if let Some(has_value) = properties.get("hasValue") {
                let individual = fukurow_lite::Individual(OwlIri::new(has_value.clone()));
                let restriction = ClassExpression::HasValue {
                    property: on_property.clone(),
                    individual,
                };
                ontology.add_class_expression(&restriction);
            }

            // Cardinality restrictions (simplified - need to parse qualified/unqualified)
            if let Some(min_card) = properties.get("minCardinality") {
                if let Ok(cardinality) = min_card.parse::<u32>() {
                    let restriction = ClassExpression::MinCardinality {
                        cardinality,
                        property: on_property.clone(),
                        class: None, // Unqualified
                    };
                    ontology.add_class_expression(&restriction);
                }
            }

            if let Some(max_card) = properties.get("maxCardinality") {
                if let Ok(cardinality) = max_card.parse::<u32>() {
                    let restriction = ClassExpression::MaxCardinality {
                        cardinality,
                        property: on_property.clone(),
                        class: None, // Unqualified
                    };
                    ontology.add_class_expression(&restriction);
                }
            }

            if let Some(exact_card) = properties.get("exactCardinality") {
                if let Ok(cardinality) = exact_card.parse::<u32>() {
                    let restriction = ClassExpression::ExactCardinality {
                        cardinality,
                        property: on_property.clone(),
                        class: None, // Unqualified
                    };
                    ontology.add_class_expression(&restriction);
                }
            }
        }

        // Process inverse properties
        for (prop_iri, inverse_iri) in inverse_properties {
            let base_prop = PropertyExpression::ObjectProperty(OwlIri::new(prop_iri.clone()));
            let inverse_prop = PropertyExpression::InverseOf(Box::new(base_prop));
            ontology.add_property_expression(&inverse_prop);

            // Add symmetric inverse relationship
            let inverse_base = PropertyExpression::ObjectProperty(OwlIri::new(inverse_iri.clone()));
            let inverse_inverse = PropertyExpression::InverseOf(Box::new(inverse_base));
            ontology.add_property_expression(&inverse_inverse);
        }

        Ok(())
    }

    /// Parse RDF list into vector of IRIs
    fn parse_rdf_list(&self, store: &RdfStore, list_head: &str) -> Option<Vec<String>> {
        let rdf_first = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
        let rdf_rest = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
        let rdf_nil = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

        let mut result = Vec::new();
        let mut current = list_head.to_string();

        loop {
            // Find first element
            let mut found_first = false;
            for stored_triple in store.all_triples().values().flatten() {
                let triple = &stored_triple.triple;
                if triple.subject == current && triple.predicate == rdf_first {
                    result.push(triple.object.clone());
                    found_first = true;
                    break;
                }
            }

            if !found_first {
                return None; // Malformed list
            }

            // Find rest of list
            let mut found_rest = false;
            for stored_triple in store.all_triples().values().flatten() {
                let triple = &stored_triple.triple;
                if triple.subject == current && triple.predicate == rdf_rest {
                    if triple.object == rdf_nil {
                        return Some(result); // End of list
                    }
                    current = triple.object.clone();
                    found_rest = true;
                    break;
                }
            }

            if !found_rest {
                return None; // Malformed list
            }
        }
    }

    /// Parse RDF list of individuals
    fn parse_individual_list(&self, store: &RdfStore, list_head: &str) -> Option<Vec<fukurow_lite::Individual>> {
        self.parse_rdf_list(store, list_head)
            .map(|iris| iris.into_iter()
                .map(|iri| fukurow_lite::Individual(OwlIri::new(iri)))
                .collect())
    }

    /// Find the property that a restriction is on
    fn find_restriction_property(&self, store: &RdfStore, restriction_iri: &str) -> Result<PropertyExpression, OwlDlError> {
        let owl_on_property = "http://www.w3.org/2002/07/owl#onProperty";

        for stored_triple in store.all_triples().values().flatten() {
            let triple = &stored_triple.triple;
            if triple.subject == restriction_iri && triple.predicate == owl_on_property {
                return Ok(PropertyExpression::ObjectProperty(OwlIri::new(triple.object.clone())));
            }
        }

        Err(OwlDlError::LoaderError(format!("No onProperty found for restriction {}", restriction_iri)))
    }

    /// Convert IRI to class expression (simplified - assumes named classes)
    fn iri_to_class_expression(&self, iri: &str) -> ClassExpression {
        if iri == "http://www.w3.org/2002/07/owl#Thing" {
            ClassExpression::Thing
        } else if iri == "http://www.w3.org/2002/07/owl#Nothing" {
            ClassExpression::Nothing
        } else {
            ClassExpression::Named(OwlIri::new(iri.to_string()))
        }
    }
}
