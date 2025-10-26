//! SHACL 制約検証

use crate::loader::{ShapesGraph, Shape, NodeShape, PropertyShape, Target, PropertyConstraint, NodeConstraint, PropertyPath};
use crate::report::{ValidationReport, ValidationResult, ViolationLevel};
use fukurow_store::store::RdfStore;
use fukurow_core::model::{Iri, Literal, Term};
use std::collections::{HashMap, HashSet};

/// Validation Configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub mode: ValidationMode,
    pub report_jsonld: bool,
}

/// Validation Mode
#[derive(Debug, Clone)]
pub enum ValidationMode {
    FailFast,  // 最初の違反で停止
    Warn,      // 違反を警告として記録し続ける
    Skip,      // 検証をスキップ
}

/// SHACL Validator trait
pub trait ShaclValidator {
    fn validate_graph(
        &self,
        shapes: &ShapesGraph,
        store: &RdfStore,
        config: &ValidationConfig
    ) -> Result<ValidationReport, ShaclError>;
}

/// Default SHACL Validator
pub struct DefaultShaclValidator;

impl ShaclValidator for DefaultShaclValidator {
    fn validate_graph(
        &self,
        shapes: &ShapesGraph,
        store: &RdfStore,
        config: &ValidationConfig
    ) -> Result<ValidationReport, ShaclError> {

        if matches!(config.mode, ValidationMode::Skip) {
            return Ok(ValidationReport {
                conforms: true,
                results: vec![],
                shapes_graph: if config.report_jsonld { Some(shapes.clone()) } else { None },
            });
        }

        let mut results = Vec::new();
        let mut conforms = true;

        // 各 Shape を検証
        for (shape_id, shape) in &shapes.shapes {
            match shape {
                Shape::Node(node_shape) => {
                    let shape_results = self.validate_node_shape(node_shape, store)?;
                    results.extend(shape_results);
                }
                Shape::Property(prop_shape) => {
                    let shape_results = self.validate_property_shape(prop_shape, store)?;
                    results.extend(shape_results);
                }
            }

            // FailFast モードの場合、違反があれば即終了
            if matches!(config.mode, ValidationMode::FailFast) && !results.is_empty() {
                conforms = false;
                break;
            }
        }

        // Warn モードでは違反があっても conforms = true
        conforms = matches!(config.mode, ValidationMode::Warn) || results.is_empty();

        Ok(ValidationReport {
            conforms,
            results,
            shapes_graph: if config.report_jsonld { Some(shapes.clone()) } else { None },
        })
    }
}

impl DefaultShaclValidator {
    fn validate_node_shape(&self, shape: &NodeShape, store: &RdfStore) -> Result<Vec<ValidationResult>, ShaclError> {
        let mut results = Vec::new();

        // ターゲットノードを取得
        let target_nodes = self.get_target_nodes(&shape.targets, store)?;

        for node in target_nodes {
            // Node constraints を検証
            for constraint in &shape.constraints {
                let constraint_results = self.validate_node_constraint(constraint, &node, store)?;
                results.extend(constraint_results);
            }

            // Property shapes を検証
            for prop_shape_id in &shape.property_shapes {
                if let Some(Shape::Property(prop_shape)) = store // TODO: shapes から取得
                    .all_triples()
                    .values()
                    .find(|t| matches!(&t.triple.subject, Term::Iri(iri) if iri == prop_shape_id))
                    .map(|_| None) { // TODO: PropertyShape 取得ロジック

                    let prop_results = self.validate_property_shape(prop_shape, store)?;
                    results.extend(prop_results);
                }
            }
        }

        Ok(results)
    }

    fn validate_property_shape(&self, shape: &PropertyShape, store: &RdfStore) -> Result<Vec<ValidationResult>, ShaclError> {
        let mut results = Vec::new();

        // TODO: Property path に従って値を取得
        let values = self.get_property_values(&shape.path, store)?;

        // Property constraints を検証
        for constraint in &shape.constraints {
            let constraint_results = self.validate_property_constraint(constraint, &values)?;
            results.extend(constraint_results);
        }

        Ok(results)
    }

    fn get_target_nodes(&self, targets: &[Target], store: &RdfStore) -> Result<HashSet<Iri>, ShaclError> {
        let mut nodes = HashSet::new();

        for target in targets {
            match target {
                Target::Class(class) => {
                    // rdf:type が class であるノードを取得
                    let rdf_type = Iri::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string());

                    for stored_triple in store.all_triples().values() {
                        let triple = &stored_triple.triple;
                        if triple.predicate == rdf_type && triple.object == Term::Iri(class.clone()) {
                            if let Term::Iri(subject_iri) = &triple.subject {
                                nodes.insert(subject_iri.clone());
                            }
                        }
                    }
                }
                Target::Node(node) => {
                    nodes.insert(node.clone());
                }
                Target::SubjectsOf(predicate) => {
                    for stored_triple in store.all_triples().values() {
                        let triple = &stored_triple.triple;
                        if triple.predicate == *predicate {
                            if let Term::Iri(subject_iri) = &triple.subject {
                                nodes.insert(subject_iri.clone());
                            }
                        }
                    }
                }
                Target::ObjectsOf(predicate) => {
                    for stored_triple in store.all_triples().values() {
                        let triple = &stored_triple.triple;
                        if triple.predicate == *predicate {
                            if let Term::Iri(object_iri) = &triple.object {
                                nodes.insert(object_iri.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(nodes)
    }

    fn validate_node_constraint(&self, constraint: &NodeConstraint, node: &Iri, store: &RdfStore) -> Result<Vec<ValidationResult>, ShaclError> {
        let mut results = Vec::new();

        match constraint {
            NodeConstraint::Class(expected_class) => {
                let rdf_type = Iri::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string());
                let mut has_class = false;

                for stored_triple in store.all_triples().values() {
                    let triple = &stored_triple.triple;
                    if triple.subject == Term::Iri(node.clone()) &&
                       triple.predicate == rdf_type &&
                       triple.object == Term::Iri(expected_class.clone()) {
                        has_class = true;
                        break;
                    }
                }

                if !has_class {
                    results.push(ValidationResult {
                        focus_node: Some(node.clone()),
                        result_path: None,
                        value: None,
                        source_constraint_component: Iri::new("http://www.w3.org/ns/shacl#class".to_string()),
                        source_shape: None, // TODO
                        detail: None,
                        message: Some(format!("Node {} is not of class {}", node, expected_class)),
                        severity: ViolationLevel::Violation,
                    });
                }
            }
            NodeConstraint::Datatype(expected_datatype) => {
                // ノードの値を取得（リテラルとして）
                let mut node_values = Vec::new();
                for stored_triple in store.all_triples().values() {
                    let triple = &stored_triple.triple;
                    if triple.subject == Term::Iri(node.clone()) {
                        if let Term::Literal(lit) = &triple.object {
                            node_values.push(lit.clone());
                        }
                    }
                }

                for value in node_values {
                    if value.datatype != Some(expected_datatype.clone()) {
                        results.push(ValidationResult {
                            focus_node: Some(node.clone()),
                            result_path: None,
                            value: Some(Term::Literal(value.clone())),
                            source_constraint_component: Iri::new("http://www.w3.org/ns/shacl#datatype".to_string()),
                            source_shape: None,
                            detail: None,
                            message: Some(format!("Value {} does not have datatype {}", value.value, expected_datatype)),
                            severity: ViolationLevel::Violation,
                        });
                    }
                }
            }
            NodeConstraint::HasValue(expected_value) => {
                let mut found = false;
                for stored_triple in store.all_triples().values() {
                    let triple = &stored_triple.triple;
                    if triple.subject == Term::Iri(node.clone()) &&
                       triple.object == Term::Literal(expected_value.clone()) {
                        found = true;
                        break;
                    }
                }

                if !found {
                    results.push(ValidationResult {
                        focus_node: Some(node.clone()),
                        result_path: None,
                        value: None,
                        source_constraint_component: Iri::new("http://www.w3.org/ns/shacl#hasValue".to_string()),
                        source_shape: None,
                        detail: None,
                        message: Some(format!("Node {} does not have required value {}", node, expected_value.value)),
                        severity: ViolationLevel::Violation,
                    });
                }
            }
            // TODO: 他の node constraints の実装
            _ => {
                // サポートされていない制約
            }
        }

        Ok(results)
    }

    fn validate_property_constraint(&self, constraint: &PropertyConstraint, values: &[Term]) -> Result<Vec<ValidationResult>, ShaclError> {
        let mut results = Vec::new();

        match constraint {
            PropertyConstraint::MinCount(min_count) => {
                if values.len() < *min_count as usize {
                    results.push(ValidationResult {
                        focus_node: None, // TODO
                        result_path: None, // TODO
                        value: None,
                        source_constraint_component: Iri::new("http://www.w3.org/ns/shacl#minCount".to_string()),
                        source_shape: None,
                        detail: None,
                        message: Some(format!("Expected at least {} values, found {}", min_count, values.len())),
                        severity: ViolationLevel::Violation,
                    });
                }
            }
            PropertyConstraint::MaxCount(max_count) => {
                if values.len() > *max_count as usize {
                    results.push(ValidationResult {
                        focus_node: None,
                        result_path: None,
                        value: None,
                        source_constraint_component: Iri::new("http://www.w3.org/ns/shacl#maxCount".to_string()),
                        source_shape: None,
                        detail: None,
                        message: Some(format!("Expected at most {} values, found {}", max_count, values.len())),
                        severity: ViolationLevel::Violation,
                    });
                }
            }
            PropertyConstraint::HasValue(expected_value) => {
                let has_value = values.iter().any(|v| v == &Term::Literal(expected_value.clone()));
                if !has_value {
                    results.push(ValidationResult {
                        focus_node: None,
                        result_path: None,
                        value: None,
                        source_constraint_component: Iri::new("http://www.w3.org/ns/shacl#hasValue".to_string()),
                        source_shape: None,
                        detail: None,
                        message: Some(format!("Required value {} not found", expected_value.value)),
                        severity: ViolationLevel::Violation,
                    });
                }
            }
            // TODO: 他の property constraints の実装
            _ => {
                // サポートされていない制約
            }
        }

        Ok(results)
    }

    fn get_property_values(&self, path: &PropertyPath, store: &RdfStore) -> Result<Vec<Term>, ShaclError> {
        // TODO: Property path に従って値を抽出
        // 簡易実装として全トリプルを返す
        Ok(store.all_triples().values().map(|t| t.triple.object.clone()).collect())
    }
}
