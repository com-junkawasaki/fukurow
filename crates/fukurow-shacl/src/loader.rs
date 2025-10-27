//! SHACL ShapesGraph 読み込み

use fukurow_sparql::parser::{Iri, Term};
use fukurow_store::store::RdfStore;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::ShaclError;

/// Shapes Graph
#[derive(Debug, Clone)]
pub struct ShapesGraph {
    pub shapes: HashMap<Iri, Shape>,
    pub prefixes: HashMap<String, String>,
}

/// Shape (Node Shape or Property Shape)
#[derive(Debug, Clone)]
pub enum Shape {
    Node(NodeShape),
    Property(PropertyShape),
}

/// Node Shape
#[derive(Debug, Clone)]
pub struct NodeShape {
    pub id: Iri,
    pub targets: Vec<Target>,
    pub constraints: Vec<NodeConstraint>,
    pub property_shapes: Vec<Iri>, // 参照される Property Shape の ID
}

/// Property Shape
#[derive(Debug, Clone)]
pub struct PropertyShape {
    pub id: Iri,
    pub path: PropertyPath,
    pub constraints: Vec<PropertyConstraint>,
}

/// Target specification
#[derive(Debug, Clone)]
pub enum Target {
    Class(Iri),
    Node(Iri),
    SubjectsOf(Iri),
    ObjectsOf(Iri),
}

/// Property Path
#[derive(Debug, Clone)]
pub enum PropertyPath {
    Predicate(Iri),
    Inverse(Box<PropertyPath>),
    Sequence(Vec<PropertyPath>),
    Alternative(Vec<PropertyPath>),
    ZeroOrMore(Box<PropertyPath>),
    OneOrMore(Box<PropertyPath>),
    ZeroOrOne(Box<PropertyPath>),
}

/// Node Constraints
#[derive(Debug, Clone)]
pub enum NodeConstraint {
    Class(Iri),
    Datatype(Iri),
    NodeKind(NodeKind),
    MinExclusive(String),
    MaxExclusive(String),
    MinInclusive(String),
    MaxInclusive(String),
    MinLength(u64),
    MaxLength(u64),
    Pattern { pattern: String, flags: Option<String> },
    LanguageIn(Vec<String>),
    UniqueLang(bool),
    Equals(Iri),
    Disjoint(Iri),
    LessThan(Iri),
    LessThanOrEquals(Iri),
    In(Vec<String>),
    HasValue(String),
    Closed { closed: bool, ignored_properties: Vec<Iri> },
}

/// Property Constraints
#[derive(Debug, Clone)]
pub enum PropertyConstraint {
    MinCount(u64),
    MaxCount(u64),
    QualifiedValueShape {
        shape: Iri,
        qualified_min_count: Option<u64>,
        qualified_max_count: Option<u64>,
        qualified_value_shapes_disjoint: Option<bool>,
    },
    UniqueLang(bool),
    LanguageIn(Vec<String>),
    Equals(Iri),
    Disjoint(Iri),
    LessThan(Iri),
    LessThanOrEquals(Iri),
    MinExclusive(String),
    MaxExclusive(String),
    MinInclusive(String),
    MaxInclusive(String),
    MinLength(u64),
    MaxLength(u64),
    Pattern { pattern: String, flags: Option<String> },
    In(Vec<String>),
    HasValue(String),
    SparqlConstraint(String), // SHACL-SPARQL
}

/// Node Kind
#[derive(Debug, Clone)]
pub enum NodeKind {
    BlankNode,
    Iri,
    String,
    BlankNodeOrIri,
    BlankNodeOrString,
    IriOrString,
}

/// SHACL Loader trait
pub trait ShaclLoader {
    fn load_turtle(&self, ttl: &str) -> Result<ShapesGraph, ShaclError>;
    fn load_jsonld(&self, jsonld: &Value) -> Result<ShapesGraph, ShaclError>;
    fn load_from_store(&self, store: &RdfStore) -> Result<ShapesGraph, ShaclError>;
}

/// Default SHACL Loader
pub struct DefaultShaclLoader;

impl ShaclLoader for DefaultShaclLoader {
    fn load_turtle(&self, ttl: &str) -> Result<ShapesGraph, ShaclError> {
        // TODO: Turtle パーサー実装
        // ここではダミー実装
        Ok(ShapesGraph {
            shapes: HashMap::new(),
            prefixes: HashMap::new(),
        })
    }

    fn load_jsonld(&self, jsonld: &Value) -> Result<ShapesGraph, ShaclError> {
        // TODO: JSON-LD から ShapesGraph 変換
        // ここではダミー実装
        Ok(ShapesGraph {
            shapes: HashMap::new(),
            prefixes: HashMap::new(),
        })
    }

    fn load_from_store(&self, store: &RdfStore) -> Result<ShapesGraph, ShaclError> {
        let mut shapes = HashMap::new();
        let mut prefixes = HashMap::new();

        // SHACL 語彙の IRI
        let sh_target_class = Iri("http://www.w3.org/ns/shacl#targetClass".to_string());
        let sh_property = Iri("http://www.w3.org/ns/shacl#property".to_string());
        let sh_path = Iri("http://www.w3.org/ns/shacl#path".to_string());
        let sh_min_count = Iri("http://www.w3.org/ns/shacl#minCount".to_string());
        let sh_max_count = Iri("http://www.w3.org/ns/shacl#maxCount".to_string());
        let sh_datatype = Iri("http://www.w3.org/ns/shacl#datatype".to_string());
        let sh_class = Iri("http://www.w3.org/ns/shacl#class".to_string());
        let sh_node_kind = Iri("http://www.w3.org/ns/shacl#nodeKind".to_string());
        let sh_pattern = Iri("http://www.w3.org/ns/shacl#pattern".to_string());
        let sh_min_length = Iri("http://www.w3.org/ns/shacl#minLength".to_string());
        let sh_max_length = Iri("http://www.w3.org/ns/shacl#maxLength".to_string());
        let sh_min_inclusive = Iri("http://www.w3.org/ns/shacl#minInclusive".to_string());
        let sh_max_inclusive = Iri("http://www.w3.org/ns/shacl#maxInclusive".to_string());
        let sh_has_value = Iri("http://www.w3.org/ns/shacl#hasValue".to_string());
        let sh_in = Iri("http://www.w3.org/ns/shacl#in".to_string());

        // ストアから Shape を構築
        for stored_triple in store.all_triples().values().flatten() {
            let triple = &stored_triple.triple;

            // targetClass 関係から Node Shape を検出
            if triple.predicate == sh_target_class.0.as_str() {
                let shape_iri = Iri(triple.subject.clone());
                let class_iri = Iri(triple.object.clone());

                let shape = shapes.entry(shape_iri.clone()).or_insert_with(|| Shape::Node(NodeShape {
                    id: shape_iri.clone(),
                    targets: vec![Target::Class(class_iri.clone())],
                    constraints: vec![],
                    property_shapes: vec![],
                }));

                if let Shape::Node(node_shape) = shape {
                    node_shape.targets.push(Target::Class(class_iri.clone()));
                }
            }

            // property 関係から Property Shape を検出
            if triple.predicate == sh_property.0.as_str() {
                let parent_shape_iri = Iri(triple.subject.clone());
                let prop_shape_iri = Iri(triple.object.clone());

                let parent_shape_entry = shapes.entry(parent_shape_iri.clone()).or_insert_with(||
                    Shape::Node(NodeShape {
                        id: parent_shape_iri.clone(),
                        targets: vec![],
                        constraints: vec![],
                        property_shapes: vec![],
                    })
                );

                if let Shape::Node(node_shape) = parent_shape_entry {
                    node_shape.property_shapes.push(prop_shape_iri.clone());
                }

                // Property Shape 自体も登録
                shapes.entry(prop_shape_iri.clone()).or_insert_with(||
                    Shape::Property(PropertyShape {
                        id: prop_shape_iri.clone(),
                        path: PropertyPath::Predicate(Iri("http://example.org/predicate".to_string())), // TODO: path 取得
                        constraints: vec![],
                    })
                );
            }

            // minCount 制約を検出
            if triple.predicate == sh_min_count.0.as_str() {
                let shape_iri = Iri(triple.subject.clone());
                let count_str = triple.object.clone();

                if let Some(count) = count_str.parse::<u64>().ok() {
                    let shape = shapes.entry(shape_iri.clone()).or_insert_with(||
                        Shape::Property(PropertyShape {
                            id: shape_iri.clone(),
                            path: PropertyPath::Predicate(Iri("http://example.org/predicate".to_string())),
                            constraints: vec![],
                        })
                    );

                    if let Shape::Property(prop_shape) = shape {
                        prop_shape.constraints.push(PropertyConstraint::MinCount(count));
                    }
                }
            }

            // maxCount 制約を検出
            if triple.predicate == sh_max_count.0.as_str() {
                let shape_iri = Iri(triple.subject.clone());
                let count_str = triple.object.clone();

                if let Some(count) = count_str.parse::<u64>().ok() {
                    let shape = shapes.entry(shape_iri.clone()).or_insert_with(||
                        Shape::Property(PropertyShape {
                            id: shape_iri.clone(),
                            path: PropertyPath::Predicate(Iri("http://example.org/predicate".to_string())),
                            constraints: vec![],
                        })
                    );

                    if let Shape::Property(prop_shape) = shape {
                        prop_shape.constraints.push(PropertyConstraint::MaxCount(count));
                    }
                }
            }

            // datatype 制約を検出
            if triple.predicate == sh_datatype.0.as_str() {
                let shape_iri = Iri(triple.subject.clone());
                let datatype_iri = Iri(triple.object.clone());

                let shape = shapes.entry(shape_iri.clone()).or_insert_with(|| Shape::Node(NodeShape {
                    id: shape_iri.clone(),
                    targets: vec![],
                    constraints: vec![],
                    property_shapes: vec![],
                }));

                if let Shape::Node(node_shape) = shape {
                    node_shape.constraints.push(NodeConstraint::Datatype(datatype_iri.clone()));
                }
            }

            // class 制約を検出
            if triple.predicate == sh_class.0.as_str() {
                let shape_iri = Iri(triple.subject.clone());
                let class_iri = Iri(triple.object.clone());

                let shape = shapes.entry(shape_iri.clone()).or_insert_with(|| Shape::Node(NodeShape {
                    id: shape_iri.clone(),
                    targets: vec![],
                    constraints: vec![],
                    property_shapes: vec![],
                }));

                if let Shape::Node(node_shape) = shape {
                    node_shape.constraints.push(NodeConstraint::Class(class_iri.clone()));
                }
            }

            // hasValue 制約を検出
            if triple.predicate == sh_has_value.0.as_str() {
                let shape_iri = Iri(triple.subject.clone());
                let value_str = triple.object.clone();

                let shape = shapes.entry(shape_iri.clone()).or_insert_with(|| Shape::Node(NodeShape {
                    id: shape_iri.clone(),
                    targets: vec![],
                    constraints: vec![],
                    property_shapes: vec![],
                }));

                if let Shape::Node(node_shape) = shape {
                    node_shape.constraints.push(NodeConstraint::HasValue(value_str.clone()));
                }
            }

            // TODO: 他の制約の読み込みを実装
        }

        Ok(ShapesGraph { shapes, prefixes })
    }
}
