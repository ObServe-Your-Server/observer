use std::sync::Arc;

pub struct StorageEngine {
    pub save_to_file_interval: u32,
    pub storage_channels: u32
}

impl StorageEngine {
    pub fn default() -> Self{
        StorageEngine {
            save_to_file_interval: 60,
            storage_channels: 1,
        }
    }
}