use std::{fs, io};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::config::config_parts::client_config::ClientConfig;
use crate::config::config_parts::interval_config::IntervalConfig;
use crate::config::config_parts::storage_config::StorageConfig;
use anyhow::Result;
use crate::config::config_parts::notification_config::NotificationConfig;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TomlConfig {
    client_config: ClientConfig,
    interval_config: IntervalConfig,
    notification_config: NotificationConfig,
    storage_config: StorageConfig
}

impl TomlConfig {
    pub fn load_from_path(file_path: &PathBuf) -> Result<TomlConfig>{
        let raw = fs::read_to_string(file_path)?;
        let toml_config: TomlConfig = toml::from_str::<TomlConfig>(&raw)?;
        Ok(toml_config)
    }
}