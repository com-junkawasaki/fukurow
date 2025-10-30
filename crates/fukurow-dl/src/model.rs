//! OWL DL データモデル

use fukurow_lite::model::{Class as OwlLiteClass, Property as OwlLiteProperty, Individual, OwlIri};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// OWL DL Class Expression (extends OWL Lite)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClassExpression {
    /// Named class (from OWL Lite)
    Named(OwlIri),

    /// owl:Thing (⊤)
    Thing,

    /// owl:Nothing (⊥)
    Nothing,

    /// Intersection of classes: C1 ⊓ C2 ⊓ ... ⊓ Cn
    IntersectionOf(Vec<ClassExpression>),

    /// Union of classes: C1 ⊔ C2 ⊔ ... ⊔ Cn
    UnionOf(Vec<ClassExpression>),

    /// Complement of class: ¬C
    ComplementOf(Box<ClassExpression>),

    /// Enumeration of individuals: {i1, i2, ..., in}
    OneOf(Vec<Individual>),

    /// Existential restriction: ∃R.C
    SomeValuesFrom {
        property: PropertyExpression,
        class: Box<ClassExpression>,
    },

    /// Universal restriction: ∀R.C
    AllValuesFrom {
        property: PropertyExpression,
        class: Box<ClassExpression>,
    },

    /// Has value: ∃R.{i}
    HasValue {
        property: PropertyExpression,
        individual: Individual,
    },

    /// Minimum cardinality: ≥n R.C
    MinCardinality {
        cardinality: u32,
        property: PropertyExpression,
        class: Option<Box<ClassExpression>>, // None means owl:Thing
    },

    /// Maximum cardinality: ≤n R.C
    MaxCardinality {
        cardinality: u32,
        property: PropertyExpression,
        class: Option<Box<ClassExpression>>, // None means owl:Thing
    },

    /// Exact cardinality: =n R.C
    ExactCardinality {
        cardinality: u32,
        property: PropertyExpression,
        class: Option<Box<ClassExpression>>, // None means owl:Thing
    },
}

/// OWL DL Property Expression
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyExpression {
    /// Object property
    ObjectProperty(OwlIri),

    /// Data property
    DataProperty(OwlIri),

    /// Inverse property: R⁻
    InverseOf(Box<PropertyExpression>),
}

/// OWL DL Axiom (extends OWL Lite)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axiom {
    /// OWL Lite axioms (re-exported)
    OwlLite(fukurow_lite::Axiom),

    /// SubClassOf with complex class expressions
    SubClassOf(ClassExpression, ClassExpression),

    /// EquivalentClasses with complex expressions
    EquivalentClasses(Vec<ClassExpression>),

    /// DisjointClasses with complex expressions
    DisjointClasses(Vec<ClassExpression>),

    /// SubPropertyOf with property expressions
    SubPropertyOf(PropertyExpression, PropertyExpression),

    /// EquivalentProperties
    EquivalentProperties(Vec<PropertyExpression>),

    /// ObjectPropertyDomain with complex class
    ObjectPropertyDomain(PropertyExpression, ClassExpression),

    /// ObjectPropertyRange with complex class
    ObjectPropertyRange(PropertyExpression, ClassExpression),

    /// Functional property
    FunctionalProperty(PropertyExpression),

    /// Inverse functional property
    InverseFunctionalProperty(PropertyExpression),

    /// Transitive property
    TransitiveProperty(PropertyExpression),

    /// Symmetric property
    SymmetricProperty(PropertyExpression),

    /// Asymmetric property
    AsymmetricProperty(PropertyExpression),

    /// Reflexive property
    ReflexiveProperty(PropertyExpression),

    /// Irreflexive property
    IrreflexiveProperty(PropertyExpression),

    /// Property disjointness
    DisjointProperties(Vec<PropertyExpression>),

    /// Same individual
    SameIndividual(Vec<Individual>),

    /// Different individuals
    DifferentIndividuals(Vec<Individual>),

    /// Class assertion with complex class
    ClassAssertion(ClassExpression, Individual),

    /// Object property assertion
    ObjectPropertyAssertion(PropertyExpression, Individual, Individual),

    /// Negative object property assertion
    NegativeObjectPropertyAssertion(PropertyExpression, Individual, Individual),

    /// Data property assertion (for completeness)
    DataPropertyAssertion(OwlIri, Individual, String), // Simplified: IRI, Individual, Value

    /// Negative data property assertion
    NegativeDataPropertyAssertion(OwlIri, Individual, String),
}

/// OWL DL Ontology (extends OWL Lite)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwlDlOntology {
    /// Ontology IRI
    pub iri: Option<OwlIri>,

    /// All axioms in the ontology
    pub axioms: Vec<Axiom>,

    /// All classes mentioned (for compatibility)
    pub classes: HashSet<OwlLiteClass>,

    /// All properties mentioned (for compatibility)
    pub properties: HashSet<OwlLiteProperty>,

    /// All individuals mentioned
    pub individuals: HashSet<Individual>,

    /// Class expressions (for OWL DL specific constructs)
    pub class_expressions: HashSet<ClassExpression>,

    /// Property expressions (for OWL DL specific constructs)
    pub property_expressions: HashSet<PropertyExpression>,
}

impl OwlDlOntology {
    pub fn new() -> Self {
        Self {
            iri: None,
            axioms: Vec::new(),
            classes: HashSet::new(),
            properties: HashSet::new(),
            individuals: HashSet::new(),
            class_expressions: HashSet::new(),
            property_expressions: HashSet::new(),
        }
    }

    pub fn with_iri(iri: OwlIri) -> Self {
        Self {
            iri: Some(iri),
            axioms: Vec::new(),
            classes: HashSet::new(),
            properties: HashSet::new(),
            individuals: HashSet::new(),
            class_expressions: HashSet::new(),
            property_expressions: HashSet::new(),
        }
    }

    pub fn add_axiom(&mut self, axiom: Axiom) {
        // Extract entities from axioms
        match &axiom {
            Axiom::OwlLite(lite_axiom) => {
                // Handle OWL Lite axioms by converting to DL format
                match lite_axiom {
                    fukurow_lite::Axiom::SubClassOf(c1, c2) => {
                        let ce1 = Self::owl_lite_class_to_expression(c1.clone());
                        let ce2 = Self::owl_lite_class_to_expression(c2.clone());
                        self.add_class_expression(&ce1);
                        self.add_class_expression(&ce2);
                    }
                    fukurow_lite::Axiom::EquivalentClasses(classes) => {
                        for class in classes {
                            let expr = Self::owl_lite_class_to_expression(class.clone());
                            self.add_class_expression(&expr);
                        }
                    }
                    fukurow_lite::Axiom::DisjointClasses(classes) => {
                        for class in classes {
                            let expr = Self::owl_lite_class_to_expression(class.clone());
                            self.add_class_expression(&expr);
                        }
                    }
                    fukurow_lite::Axiom::SubPropertyOf(p1, p2) => {
                        let pe1 = Self::owl_lite_property_to_expression(p1.clone());
                        let pe2 = Self::owl_lite_property_to_expression(p2.clone());
                        self.add_property_expression(&pe1);
                        self.add_property_expression(&pe2);
                    }
                    fukurow_lite::Axiom::EquivalentProperties(properties) => {
                        for prop in properties {
                            let expr = Self::owl_lite_property_to_expression(prop.clone());
                            self.add_property_expression(&expr);
                        }
                    }
                    fukurow_lite::Axiom::ObjectPropertyDomain(p, c) => {
                        let pe = Self::owl_lite_property_to_expression(p.clone());
                        let ce = Self::owl_lite_class_to_expression(c.clone());
                        self.add_property_expression(&pe);
                        self.add_class_expression(&ce);
                    }
                    fukurow_lite::Axiom::ObjectPropertyRange(p, c) => {
                        let pe = Self::owl_lite_property_to_expression(p.clone());
                        let ce = Self::owl_lite_class_to_expression(c.clone());
                        self.add_property_expression(&pe);
                        self.add_class_expression(&ce);
                    }
                    fukurow_lite::Axiom::FunctionalProperty(p) |
                    fukurow_lite::Axiom::InverseFunctionalProperty(p) |
                    fukurow_lite::Axiom::TransitiveProperty(p) |
                    fukurow_lite::Axiom::SymmetricProperty(p) => {
                        let pe = Self::owl_lite_property_to_expression(p.clone());
                        self.add_property_expression(&pe);
                    }
                    fukurow_lite::Axiom::SameIndividual(individuals) => {
                        self.individuals.extend(individuals.iter().cloned());
                    }
                    fukurow_lite::Axiom::DifferentIndividuals(individuals) => {
                        self.individuals.extend(individuals.iter().cloned());
                    }
                    fukurow_lite::Axiom::ClassAssertion(c, i) => {
                        let ce = Self::owl_lite_class_to_expression(c.clone());
                        self.add_class_expression(&ce);
                        self.individuals.insert(i.clone());
                    }
                    fukurow_lite::Axiom::ObjectPropertyAssertion(p, i1, i2) => {
                        let pe = Self::owl_lite_property_to_expression(p.clone());
                        self.add_property_expression(&pe);
                        self.individuals.insert(i1.clone());
                        self.individuals.insert(i2.clone());
                    }
                    fukurow_lite::Axiom::NegativeObjectPropertyAssertion(p, i1, i2) => {
                        let pe = Self::owl_lite_property_to_expression(p.clone());
                        self.add_property_expression(&pe);
                        self.individuals.insert(i1.clone());
                        self.individuals.insert(i2.clone());
                    }
                }
            }
            Axiom::SubClassOf(ce1, ce2) => {
                self.add_class_expression(ce1);
                self.add_class_expression(ce2);
            }
            Axiom::EquivalentClasses(expressions) => {
                for expr in expressions {
                    self.add_class_expression(expr);
                }
            }
            Axiom::DisjointClasses(expressions) => {
                for expr in expressions {
                    self.add_class_expression(expr);
                }
            }
            Axiom::SubPropertyOf(pe1, pe2) => {
                self.add_property_expression(pe1);
                self.add_property_expression(pe2);
            }
            Axiom::EquivalentProperties(expressions) => {
                for expr in expressions {
                    self.add_property_expression(expr);
                }
            }
            Axiom::ObjectPropertyDomain(pe, ce) => {
                self.add_property_expression(pe);
                self.add_class_expression(ce);
            }
            Axiom::ObjectPropertyRange(pe, ce) => {
                self.add_property_expression(pe);
                self.add_class_expression(ce);
            }
            Axiom::FunctionalProperty(pe) |
            Axiom::InverseFunctionalProperty(pe) |
            Axiom::TransitiveProperty(pe) |
            Axiom::SymmetricProperty(pe) |
            Axiom::AsymmetricProperty(pe) |
            Axiom::ReflexiveProperty(pe) |
            Axiom::IrreflexiveProperty(pe) => {
                self.add_property_expression(pe);
            }
            Axiom::DisjointProperties(expressions) => {
                for expr in expressions {
                    self.add_property_expression(expr);
                }
            }
            Axiom::SameIndividual(individuals) => {
                self.individuals.extend(individuals.iter().cloned());
            }
            Axiom::DifferentIndividuals(individuals) => {
                self.individuals.extend(individuals.iter().cloned());
            }
            Axiom::ClassAssertion(ce, i) => {
                self.add_class_expression(ce);
                self.individuals.insert(i.clone());
            }
            Axiom::ObjectPropertyAssertion(pe, i1, i2) |
            Axiom::NegativeObjectPropertyAssertion(pe, i1, i2) => {
                self.add_property_expression(pe);
                self.individuals.insert(i1.clone());
                self.individuals.insert(i2.clone());
            }
            Axiom::DataPropertyAssertion(_, i, _) |
            Axiom::NegativeDataPropertyAssertion(_, i, _) => {
                self.individuals.insert(i.clone());
            }
        }

        self.axioms.push(axiom);
    }

    fn owl_lite_class_to_expression(class: OwlLiteClass) -> ClassExpression {
        match class {
            OwlLiteClass::Named(iri) => ClassExpression::Named(iri),
            OwlLiteClass::Thing => ClassExpression::Thing,
            OwlLiteClass::Nothing => ClassExpression::Nothing,
        }
    }

    fn owl_lite_property_to_expression(property: OwlLiteProperty) -> PropertyExpression {
        match property {
            OwlLiteProperty::Object(iri) => PropertyExpression::ObjectProperty(iri),
            OwlLiteProperty::Data(iri) => PropertyExpression::DataProperty(iri),
        }
    }

    pub fn add_class_expression(&mut self, expr: &ClassExpression) {
        // Recursively collect all class expressions
        self.collect_class_expressions(expr);
    }

    pub fn add_property_expression(&mut self, expr: &PropertyExpression) {
        // Recursively collect all property expressions
        self.collect_property_expressions(expr);
    }

    fn collect_class_expressions(&mut self, expr: &ClassExpression) {
        match expr {
            ClassExpression::Named(iri) => {
                self.classes.insert(OwlLiteClass::Named(iri.clone()));
            }
            ClassExpression::Thing => {
                self.classes.insert(OwlLiteClass::Thing);
            }
            ClassExpression::Nothing => {
                self.classes.insert(OwlLiteClass::Nothing);
            }
            ClassExpression::IntersectionOf(expressions) |
            ClassExpression::UnionOf(expressions) => {
                for expr in expressions {
                    self.collect_class_expressions(expr);
                }
            }
            ClassExpression::ComplementOf(expr) => {
                self.collect_class_expressions(expr);
            }
            ClassExpression::OneOf(individuals) => {
                self.individuals.extend(individuals.iter().cloned());
            }
            ClassExpression::SomeValuesFrom { class, .. } |
            ClassExpression::AllValuesFrom { class, .. } => {
                self.collect_class_expressions(class);
            }
            ClassExpression::HasValue { individual, .. } => {
                self.individuals.insert(individual.clone());
            }
            ClassExpression::MinCardinality { class, .. } |
            ClassExpression::MaxCardinality { class, .. } |
            ClassExpression::ExactCardinality { class, .. } => {
                if let Some(class) = class {
                    self.collect_class_expressions(class);
                }
            }
        }
        self.class_expressions.insert(expr.clone());
    }

    fn collect_property_expressions(&mut self, expr: &PropertyExpression) {
        match expr {
            PropertyExpression::ObjectProperty(iri) => {
                self.properties.insert(OwlLiteProperty::Object(iri.clone()));
            }
            PropertyExpression::DataProperty(iri) => {
                self.properties.insert(OwlLiteProperty::Data(iri.clone()));
            }
            PropertyExpression::InverseOf(expr) => {
                self.collect_property_expressions(expr);
            }
        }
        self.property_expressions.insert(expr.clone());
    }
}
