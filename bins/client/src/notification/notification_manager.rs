use std::sync::Arc;
use crate::storage_engine::storage_engine::StorageEngine;

pub struct NotificationManager{
    storage_engine: Arc<StorageEngine>,
}