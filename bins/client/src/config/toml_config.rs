use crate::config::config_parts::client_config::ClientConfig;
use crate::config::config_parts::interval_config::IntervalConfig;
use crate::config::config_parts::notification_config::NotificationConfig;
use crate::config::config_parts::storage_config::StorageConfig;
use anyhow::{Ok, Result, anyhow};
use getset::{Getters, MutGetters};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tonic::Request;

#[derive(Debug, Clone, Deserialize, Serialize, Getters, MutGetters)]
#[serde(rename_all = "snake_case")]
pub struct TomlConfig {
    #[getset(get = "pub", get_mut = "pub")]
    client_config: ClientConfig,
    #[getset(get = "pub")]
    interval_config: IntervalConfig,
    #[getset(get = "pub")]
    notification_config: NotificationConfig,
    #[getset(get = "pub")]
    storage_config: StorageConfig,
}

impl TomlConfig {
    pub fn load_from_path(file_path: &PathBuf) -> Result<TomlConfig> {
        let raw = fs::read_to_string(file_path)?;
        let toml_config: TomlConfig = toml::from_str::<TomlConfig>(&raw)?;
        Self::validate_config(&toml_config)?;
        Ok(toml_config)
    }

    fn validate_config(toml_config: &TomlConfig) -> Result<()> {
        // TODO

        // storage config
        let storage_config = &toml_config.storage_config;
        if storage_config.keep_every_x_metrics <= 0 {
            return Err(anyhow!(
                "The keeping every Xth entry needs to be bigger than 0"
            ));
        }
        Ok(())
    }
}
