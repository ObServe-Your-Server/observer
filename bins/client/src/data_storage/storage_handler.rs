use std::sync::Arc;

pub struct StorageHandler{
    pub save_to_file_interval: u32,
    pub storage_channels: u32
}

impl StorageHandler {
    pub fn default() -> Self{
        StorageHandler{
            save_to_file_interval: 60,
            storage_channels: 1,
        }
    }
}