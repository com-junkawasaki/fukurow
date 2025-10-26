#[cfg(feature = "turso")]
mod inner {
    use anyhow::Result;
    use async_trait::async_trait;
    use libsql::Database;
    use chrono::{DateTime, Utc};

    use crate::adapter::StoreAdapter;
    use crate::RdfStore;
    use crate::fukurow_core::model::Triple;
    use crate::{StoredTriple, GraphId, Provenance};

    pub struct TursoAdapter {
        db: Database,
    }

    impl TursoAdapter {
        pub async fn new(database_url: String) -> Result<Self> {
            let db = Database::open(database_url).await?;
            let conn = db.connect()?;

            // schema
            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS triples (
                  graph_kind INTEGER NOT NULL,
                  graph_name TEXT,
                  s TEXT NOT NULL,
                  p TEXT NOT NULL,
                  o TEXT NOT NULL,
                  asserted_at TEXT NOT NULL,
                  provenance_json TEXT NOT NULL
                );
            "#,
                (),
            ).await?;

            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_sp ON triples(s,p);",
                (),
            ).await?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_po ON triples(p,o);",
                (),
            ).await?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_o ON triples(o);",
                (),
            ).await?;

            Ok(Self { db })
        }
    }

    #[async_trait]
    impl StoreAdapter for TursoAdapter {
        async fn save_store(&self, store: &RdfStore) -> Result<()> {
            let conn = self.db.connect()?;

            // Clear existing data
            conn.execute("DELETE FROM triples", ()).await?;

            // Insert all triples
            for (graph_id, graph) in store.all_triples() {
                for st in graph {
                    let (graph_kind, graph_name) = match graph_id {
                        GraphId::Default => (0_i64, None::<String>),
                        GraphId::Named(n) => (1_i64, Some(n.clone())),
                        GraphId::Sensor(n) => (2_i64, Some(n.clone())),
                        GraphId::Inferred(n) => (3_i64, Some(n.clone())),
                    };
                    conn.execute(
                        r#"
                        INSERT INTO triples(graph_kind, graph_name, s, p, o, asserted_at, provenance_json)
                        VALUES(?, ?, ?, ?, ?, ?, ?)
                    "#,
                        libsql::params!(
                            graph_kind,
                            graph_name,
                            &st.triple.subject,
                            &st.triple.predicate,
                            &st.triple.object,
                            st.asserted_at.to_rfc3339(),
                            serde_json::to_string(&st.provenance)?
                        ),
                    ).await?;
                }
            }

            Ok(())
        }

        async fn load_store(&self) -> Result<RdfStore> {
            let conn = self.db.connect()?;
            let mut rows = conn.query("SELECT graph_kind, graph_name, s, p, o, asserted_at, provenance_json FROM triples", ()).await?;

            let mut store = RdfStore::new();
            while let Some(row) = rows.next().await? {
                let graph_kind: i64 = row.get(0)?;
                let graph_name: Option<String> = row.get(1)?;
                let s: String = row.get(2)?;
                let p: String = row.get(3)?;
                let o: String = row.get(4)?;
                let asserted_at_str: String = row.get(5)?;
                let provenance_json: String = row.get(6)?;

                let graph_id = match (graph_kind, graph_name) {
                    (0, _) => GraphId::Default,
                    (1, Some(n)) => GraphId::Named(n),
                    (2, Some(n)) => GraphId::Sensor(n),
                    (3, Some(n)) => GraphId::Inferred(n),
                    _ => GraphId::Default,
                };
                let provenance: Provenance = serde_json::from_str(&provenance_json)?;
                let asserted_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&asserted_at_str)?.with_timezone(&Utc);

                store.insert(
                    Triple { subject: s, predicate: p, object: o },
                    graph_id,
                    provenance,
                );
                // Note: asserted_at is not directly settable; if needed we'll extend API later.
            }

            Ok(store)
        }
    }
}

#[cfg(feature = "turso")]
pub use inner::TursoAdapter;


