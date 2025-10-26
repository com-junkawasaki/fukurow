#[cfg(feature = "turso")]
mod inner {
    use anyhow::Result;
    use async_trait::async_trait;
    use crate::adapter::StoreAdapter;
    use crate::RdfStore;

    pub struct TursoAdapter {
        database_url: String,
    }

    impl TursoAdapter {
        pub fn new(database_url: String) -> Self {
            Self { database_url }
        }
    }

    #[async_trait]
    impl StoreAdapter for TursoAdapter {
        async fn save_store(&self, _store: &RdfStore) -> Result<()> {
            // TODO: implement using libsql client
            Err(anyhow::anyhow!("Turso adapter not implemented yet"))
        }

        async fn load_store(&self) -> Result<RdfStore> {
            Err(anyhow::anyhow!("Turso adapter not implemented yet"))
        }
    }
}


