use crate::storage_engine::storage_engine::StorageEngine;
use std::sync::Arc;

pub struct NotificationManager {
    storage_engine: Arc<StorageEngine>,
}
