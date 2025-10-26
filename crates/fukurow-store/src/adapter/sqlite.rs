use anyhow::Result;
use async_trait::async_trait;
use sqlx::{SqlitePool, FromRow};
use chrono::{DateTime, Utc};

use crate::{RdfStore, StoredTriple, GraphId, Provenance};
use crate::adapter::StoreAdapter;

pub struct SqliteAdapter {
    pool: SqlitePool,
}

impl SqliteAdapter {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        // schema
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS triples (
              graph_kind INTEGER NOT NULL,
              graph_name TEXT,
              s TEXT NOT NULL,
              p TEXT NOT NULL,
              o TEXT NOT NULL,
              asserted_at TEXT NOT NULL,
              provenance_json TEXT NOT NULL
            );
        "#).execute(&pool).await?;

        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_sp ON triples(s,p);"#).execute(&pool).await?;
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_po ON triples(p,o);"#).execute(&pool).await?;
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_o ON triples(o);"#).execute(&pool).await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl StoreAdapter for SqliteAdapter {
    async fn save_store(&self, store: &RdfStore) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM triples").execute(&mut *tx).await?;

        for (graph_id, graph) in store.all_triples() {
            for st in graph {
                let (graph_kind, graph_name) = match graph_id {
                    GraphId::Default => (0_i64, None),
                    GraphId::Named(n) => (1_i64, Some(n.clone())),
                    GraphId::Sensor(n) => (2_i64, Some(n.clone())),
                    GraphId::Inferred(n) => (3_i64, Some(n.clone())),
                };
                sqlx::query(r#"
                    INSERT INTO triples(graph_kind, graph_name, s, p, o, asserted_at, provenance_json)
                    VALUES(?, ?, ?, ?, ?, ?, ?)
                "#)
                .bind(graph_kind)
                .bind(graph_name)
                .bind(&st.triple.subject)
                .bind(&st.triple.predicate)
                .bind(&st.triple.object)
                .bind(st.asserted_at.to_rfc3339())
                .bind(serde_json::to_string(&st.provenance)?)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn load_store(&self) -> Result<RdfStore> {
        let rows = sqlx::query_as::<_, RowTriple>(r#"
            SELECT graph_kind, graph_name, s, p, o, asserted_at, provenance_json FROM triples
        "#)
        .fetch_all(&self.pool)
        .await?;

        let mut store = RdfStore::new();
        for r in rows {
            let graph_id = match (r.graph_kind, r.graph_name) {
                (0, _) => GraphId::Default,
                (1, Some(n)) => GraphId::Named(n),
                (2, Some(n)) => GraphId::Sensor(n),
                (3, Some(n)) => GraphId::Inferred(n),
                _ => GraphId::Default,
            };
            let provenance: Provenance = serde_json::from_str(&r.provenance_json)?;
            let asserted_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&r.asserted_at)?.with_timezone(&Utc);
            // recreate StoredTriple using public API
            store.insert(
                fukurow_core::model::Triple { subject: r.s, predicate: r.p, object: r.o },
                graph_id,
                provenance,
            );
            // Note: asserted_at is not directly settable; if needed we'll extend API later.
        }
        Ok(store)
    }
}

#[derive(FromRow)]
struct RowTriple {
    graph_kind: i64,
    graph_name: Option<String>,
    s: String,
    p: String,
    o: String,
    asserted_at: String,
    provenance_json: String,
}


