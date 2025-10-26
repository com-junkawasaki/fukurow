//! OWL Lite データモデル

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// OWL IRI wrapper for type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct OwlIri(pub String);

impl OwlIri {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OwlIri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// OWL Class
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Class {
    /// Named class
    Named(OwlIri),
    /// owl:Thing (⊤)
    Thing,
    /// owl:Nothing (⊥)
    Nothing,
}

/// OWL Property
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Property {
    /// Object property
    Object(OwlIri),
    /// Data property
    Data(OwlIri),
}

/// OWL Individual
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Individual(pub OwlIri);

/// OWL Axiom (OWL Lite subset)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axiom {
    /// SubClassOf(C1 C2)
    SubClassOf(Class, Class),

    /// EquivalentClasses(C1 ... Cn)
    EquivalentClasses(Vec<Class>),

    /// DisjointClasses(C1 ... Cn)
    DisjointClasses(Vec<Class>),

    /// SubPropertyOf(P1 P2)
    SubPropertyOf(Property, Property),

    /// EquivalentProperties(P1 ... Pn)
    EquivalentProperties(Vec<Property>),

    /// ObjectPropertyDomain(P C)
    ObjectPropertyDomain(Property, Class),

    /// ObjectPropertyRange(P C)
    ObjectPropertyRange(Property, Class),

    /// FunctionalProperty(P)
    FunctionalProperty(Property),

    /// InverseFunctionalProperty(P)
    InverseFunctionalProperty(Property),

    /// TransitiveProperty(P)
    TransitiveProperty(Property),

    /// SymmetricProperty(P)
    SymmetricProperty(Property),

    /// SameIndividual(i1 ... in)
    SameIndividual(Vec<Individual>),

    /// DifferentIndividuals(i1 ... in)
    DifferentIndividuals(Vec<Individual>),

    /// ClassAssertion(C i)
    ClassAssertion(Class, Individual),

    /// ObjectPropertyAssertion(P i1 i2)
    ObjectPropertyAssertion(Property, Individual, Individual),

    /// NegativeObjectPropertyAssertion(P i1 i2)
    NegativeObjectPropertyAssertion(Property, Individual, Individual),
}

/// OWL Ontology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ontology {
    /// Ontology IRI
    pub iri: Option<OwlIri>,

    /// All axioms in the ontology
    pub axioms: Vec<Axiom>,

    /// All classes mentioned in the ontology
    pub classes: HashSet<Class>,

    /// All properties mentioned in the ontology
    pub properties: HashSet<Property>,

    /// All individuals mentioned in the ontology
    pub individuals: HashSet<Individual>,
}

impl Ontology {
    pub fn new() -> Self {
        Self {
            iri: None,
            axioms: Vec::new(),
            classes: HashSet::new(),
            properties: HashSet::new(),
            individuals: HashSet::new(),
        }
    }

    pub fn with_iri(iri: OwlIri) -> Self {
        Self {
            iri: Some(iri),
            axioms: Vec::new(),
            classes: HashSet::new(),
            properties: HashSet::new(),
            individuals: HashSet::new(),
        }
    }

    pub fn add_axiom(&mut self, axiom: Axiom) {
        // Extract classes, properties, and individuals from the axiom
        match &axiom {
            Axiom::SubClassOf(c1, c2) => {
                self.classes.insert(c1.clone());
                self.classes.insert(c2.clone());
            }
            Axiom::EquivalentClasses(classes) => {
                self.classes.extend(classes.iter().cloned());
            }
            Axiom::DisjointClasses(classes) => {
                self.classes.extend(classes.iter().cloned());
            }
            Axiom::SubPropertyOf(p1, p2) => {
                self.properties.insert(p1.clone());
                self.properties.insert(p2.clone());
            }
            Axiom::EquivalentProperties(properties) => {
                self.properties.extend(properties.iter().cloned());
            }
            Axiom::ObjectPropertyDomain(p, c) => {
                self.properties.insert(p.clone());
                self.classes.insert(c.clone());
            }
            Axiom::ObjectPropertyRange(p, c) => {
                self.properties.insert(p.clone());
                self.classes.insert(c.clone());
            }
            Axiom::FunctionalProperty(p) |
            Axiom::InverseFunctionalProperty(p) |
            Axiom::TransitiveProperty(p) |
            Axiom::SymmetricProperty(p) => {
                self.properties.insert(p.clone());
            }
            Axiom::SameIndividual(individuals) => {
                self.individuals.extend(individuals.iter().cloned());
            }
            Axiom::DifferentIndividuals(individuals) => {
                self.individuals.extend(individuals.iter().cloned());
            }
            Axiom::ClassAssertion(c, i) => {
                self.classes.insert(c.clone());
                self.individuals.insert(i.clone());
            }
            Axiom::ObjectPropertyAssertion(p, i1, i2) |
            Axiom::NegativeObjectPropertyAssertion(p, i1, i2) => {
                self.properties.insert(p.clone());
                self.individuals.insert(i1.clone());
                self.individuals.insert(i2.clone());
            }
        }

        self.axioms.push(axiom);
    }
}
