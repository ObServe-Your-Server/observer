use crate::error_handling::error::Error;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait ErrorStoreStorageEngine: Send + Sync {
    async fn save_error(&self, error: Error) -> Result<()>;
}

pub struct ErrorStore {
    storage_engine: Arc<dyn ErrorStoreStorageEngine>,
}

impl ErrorStore {
    pub fn new(storage_engine: Arc<dyn ErrorStoreStorageEngine>) -> ErrorStore {
        ErrorStore { storage_engine }
    }

    pub async fn save_error(&self, error: Error) -> Result<()> {
        self.storage_engine.save_error(error).await
    }
}
