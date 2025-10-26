use async_trait::async_trait;
use anyhow::Result;

use crate::{RdfStore};

#[async_trait]
pub trait StoreAdapter: Send + Sync {
    async fn save_store(&self, store: &RdfStore) -> Result<()>;
    async fn load_store(&self) -> Result<RdfStore>;
}

pub mod sqlite;
#[cfg(feature = "turso")]
pub mod turso;


