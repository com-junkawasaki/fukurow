//! RDFS (RDF Schema) 推論エンジン
//!
//! このクレートは RDFS の推論規則を実装します:
//! - rdfs:subClassOf の推移的閉包
//! - rdfs:subPropertyOf の推移的閉包
//! - rdfs:domain と rdfs:range の推論
//! - rdf:type の推論

use fukurow_core::model::Triple;
use fukurow_store::store::RdfStore;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

/// RDF IRI wrapper for type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Iri(pub String);

impl Iri {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Iri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// RDFS 語彙の IRI
pub mod vocabulary {
    use super::Iri;

    pub const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
    pub const RDFS_SUBPROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
    pub const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
    pub const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
    pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    pub const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
    pub const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";

    pub fn rdfs_subclass_of() -> Iri { Iri::new(RDFS_SUBCLASS_OF.to_string()) }
    pub fn rdfs_subproperty_of() -> Iri { Iri::new(RDFS_SUBPROPERTY_OF.to_string()) }
    pub fn rdfs_domain() -> Iri { Iri::new(RDFS_DOMAIN.to_string()) }
    pub fn rdfs_range() -> Iri { Iri::new(RDFS_RANGE.to_string()) }
    pub fn rdf_type() -> Iri { Iri::new(RDF_TYPE.to_string()) }
}

/// RDFS 推論エンジン
#[derive(Debug)]
pub struct RdfsReasoner {
    /// クラス階層: 子クラス -> 親クラス集合
    class_hierarchy: HashMap<Iri, HashSet<Iri>>,
    /// プロパティ階層: 子プロパティ -> 親プロパティ集合
    property_hierarchy: HashMap<Iri, HashSet<Iri>>,
    /// ドメイン制約: プロパティ -> クラス
    domain_constraints: HashMap<Iri, Iri>,
    /// レンジ制約: プロパティ -> クラス
    range_constraints: HashMap<Iri, Iri>,
    /// 推論されたトリプルのキャッシュ
    inferred_triples: HashSet<Triple>,
}

impl RdfsReasoner {
    /// 新しい RDFS 推論エンジンを作成
    pub fn new() -> Self {
        Self {
            class_hierarchy: HashMap::new(),
            property_hierarchy: HashMap::new(),
            domain_constraints: HashMap::new(),
            range_constraints: HashMap::new(),
            inferred_triples: HashSet::new(),
        }
    }

    /// ストアから RDFS 知識を読み込んで推論を実行
    pub fn compute_closure(&mut self, store: &RdfStore) -> Result<Vec<Triple>, RdfsError> {
        self.load_knowledge(store);
        self.compute_transitive_closure();
        self.infer_types_and_constraints(store);

        Ok(self.inferred_triples.iter().cloned().collect())
    }

    /// RDFS 知識をストアから読み込み
    fn load_knowledge(&mut self, store: &RdfStore) {
        for stored_triple_vec in store.all_triples().values() {
            for stored_triple in stored_triple_vec {
                let triple = &stored_triple.triple;

                // rdfs:subClassOf 関係を読み込み
                if triple.predicate == vocabulary::rdfs_subclass_of().as_str() {
                    self.class_hierarchy
                        .entry(Iri::new(triple.subject.clone()))
                        .or_insert_with(HashSet::new)
                        .insert(Iri::new(triple.object.clone()));
                }

                // rdfs:subPropertyOf 関係を読み込み
                if triple.predicate == vocabulary::rdfs_subproperty_of().as_str() {
                    self.property_hierarchy
                        .entry(Iri::new(triple.subject.clone()))
                        .or_insert_with(HashSet::new)
                        .insert(Iri::new(triple.object.clone()));
                }

                // rdfs:domain 制約を読み込み
                if triple.predicate == vocabulary::rdfs_domain().as_str() {
                    self.domain_constraints.insert(
                        Iri::new(triple.subject.clone()),
                        Iri::new(triple.object.clone()),
                    );
                }

                // rdfs:range 制約を読み込み
                if triple.predicate == vocabulary::rdfs_range().as_str() {
                    self.range_constraints.insert(
                        Iri::new(triple.subject.clone()),
                        Iri::new(triple.object.clone()),
                    );
                }
            }
        }
    }

    /// 推移的閉包を計算
    fn compute_transitive_closure(&mut self) {
        // クラス階層の推移的閉包
        let class_hierarchy_input = self.class_hierarchy.clone();
        Self::compute_hierarchy_closure(&class_hierarchy_input, &mut self.class_hierarchy);

        // プロパティ階層の推移的閉包
        let property_hierarchy_input = self.property_hierarchy.clone();
        Self::compute_hierarchy_closure(&property_hierarchy_input, &mut self.property_hierarchy);

        // 推論されたトリプルを生成
        for (child, parents) in &self.class_hierarchy {
            for parent in parents {
                if child != parent {  // 自己参照は除く
                    self.inferred_triples.insert(Triple {
                        subject: child.0.clone(),
                        predicate: vocabulary::rdfs_subclass_of().as_str().to_string(),
                        object: parent.0.clone(),
                    });
                }
            }
        }

        for (child, parents) in &self.property_hierarchy {
            for parent in parents {
                if child != parent {  // 自己参照は除く
                    self.inferred_triples.insert(Triple {
                        subject: child.0.clone(),
                        predicate: vocabulary::rdfs_subproperty_of().as_str().to_string(),
                        object: parent.0.clone(),
                    });
                }
            }
        }
    }

    /// 階層関係の推移的閉包を計算
    fn compute_hierarchy_closure(input: &HashMap<Iri, HashSet<Iri>>, output: &mut HashMap<Iri, HashSet<Iri>>) {
        // Create a copy of input for iteration to avoid borrow conflicts
        let input_copy = input.clone();
        let mut changed = true;
        while changed {
            changed = false;
            for (child, direct_parents) in &input_copy {
                let mut all_parents = output.get(child).cloned().unwrap_or_default();

                for direct_parent in direct_parents {
                    if let Some(grand_parents) = input_copy.get(direct_parent) {
                        for grand_parent in grand_parents {
                            if all_parents.insert(grand_parent.clone()) {
                                changed = true;
                            }
                        }
                    }
                }

                output.insert(child.clone(), all_parents);
            }
        }
    }

    /// 型推論と制約に基づく推論を実行
    fn infer_types_and_constraints(&mut self, store: &RdfStore) {
        // ドメイン制約に基づく rdf:type 推論
        for (property, class) in &self.domain_constraints {
            // このプロパティを使用している全ての主語に対して型を推論
            for stored_triple_vec in store.all_triples().values() {
                for stored_triple in stored_triple_vec {
                    let triple = &stored_triple.triple;
                    if triple.predicate == property.as_str() {
                        self.inferred_triples.insert(Triple {
                            subject: triple.subject.clone(),
                            predicate: vocabulary::rdf_type().as_str().to_string(),
                            object: class.0.clone(),
                        });
                    }
                }
            }
        }

        // レンジ制約に基づく rdf:type 推論
        for (property, class) in &self.range_constraints {
            // このプロパティを使用している全ての目的語に対して型を推論
            for stored_triple_vec in store.all_triples().values() {
                for stored_triple in stored_triple_vec {
                    let triple = &stored_triple.triple;
                    if triple.predicate == property.as_str() {
                        self.inferred_triples.insert(Triple {
                            subject: triple.object.clone(),
                            predicate: vocabulary::rdf_type().as_str().to_string(),
                            object: class.0.clone(),
                        });
                    }
                }
            }
        }

        // クラス階層に基づく rdf:type 推論
        // もし x rdf:type A であり A rdfs:subClassOf B なら x rdf:type B
        let mut type_inferences = Vec::new();
        for stored_triple_vec in store.all_triples().values() {
            for stored_triple in stored_triple_vec {
                let triple = &stored_triple.triple;
                if triple.predicate == vocabulary::rdf_type().as_str() {
                    let subject_iri = Iri::new(triple.subject.clone());
                    let class_iri = Iri::new(triple.object.clone());
                    if let Some(superclasses) = self.class_hierarchy.get(&class_iri) {
                        for superclass in superclasses {
                            type_inferences.push(Triple {
                                subject: subject_iri.0.clone(),
                                predicate: vocabulary::rdf_type().as_str().to_string(),
                                object: superclass.0.clone(),
                            });
                        }
                    }
                }
            }
        }

        // 型推論結果を追加
        for triple in type_inferences {
            self.inferred_triples.insert(triple);
        }
    }

    /// 推論されたトリプルを取得
    pub fn get_inferred_triples(&self) -> &HashSet<Triple> {
        &self.inferred_triples
    }

    /// クラス階層を取得
    pub fn get_class_hierarchy(&self) -> &HashMap<Iri, HashSet<Iri>> {
        &self.class_hierarchy
    }

    /// プロパティ階層を取得
    pub fn get_property_hierarchy(&self) -> &HashMap<Iri, HashSet<Iri>> {
        &self.property_hierarchy
    }
}

/// RDFS 推論の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdfsConfig {
    /// 推論の最大反復回数
    pub max_iterations: usize,
    /// 推論のタイムアウト（ミリ秒）
    pub timeout_ms: u64,
}

impl Default for RdfsConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            timeout_ms: 30000, // 30秒
        }
    }
}

/// RDFS 推論エラー
#[derive(thiserror::Error, Debug)]
pub enum RdfsError {
    #[error("Inference timeout after {0}ms")]
    Timeout(u64),

    #[error("Maximum iterations ({0}) exceeded")]
    MaxIterationsExceeded(usize),

    #[error("Invalid RDFS triple: {0}")]
    InvalidTriple(String),

    #[error("Store error: {0}")]
    StoreError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_core::model::Triple;

    #[test]
    fn test_class_hierarchy_closure() {
        let mut reasoner = RdfsReasoner::new();

        // A subclassOf B, B subclassOf C の知識を追加
        reasoner.class_hierarchy.insert(
            Iri::new("http://example.org/A".to_string()),
            HashSet::from([Iri::new("http://example.org/B".to_string())])
        );
        reasoner.class_hierarchy.insert(
            Iri::new("http://example.org/B".to_string()),
            HashSet::from([Iri::new("http://example.org/C".to_string())])
        );

        // 推移的閉包を計算
        reasoner.compute_transitive_closure();

        // A は C のサブクラスであるべき
        assert!(reasoner.class_hierarchy
            .get(&Iri::new("http://example.org/A".to_string()))
            .unwrap()
            .contains(&Iri::new("http://example.org/C".to_string())));
    }

    #[test]
    fn test_empty_reasoner() {
        let mut reasoner = RdfsReasoner::new();
        let store = RdfStore::new();

        let result = reasoner.compute_closure(&store);
        assert!(result.is_ok());

        let triples = result.unwrap();
        assert!(triples.is_empty());
    }
}
