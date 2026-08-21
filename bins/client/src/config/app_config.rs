use crate::config::config_parts::interval_config::IntervalConfig;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use getset::Getters;
use reqwest::{Client, StatusCode};
use crate::config::config_parts::client_config::ClientConfig;
use crate::config::toml_config::TomlConfig;

#[derive(Debug, Serialize, Getters)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    #[getset(get = "pub")]
    version: &'static str,
    #[getset(get = "pub")]
    toml_config: TomlConfig,
}

impl AppConfig {
    pub fn load_from_path(path: PathBuf) -> Result<AppConfig> {
        let config = AppConfig {
            version: env!("CARGO_PKG_VERSION"),
            toml_config: TomlConfig::load_from_path(&path)?
        };

        log::debug!("Config loaded from '{:?}'", path);
        log::debug!("Config: {:?}", config);

        Ok(config)
    }

    pub async fn resolve_machine_name(&mut self) -> Result<()> {
        if self.toml_config.client_config().machine_name().is_some() {
            log::debug!("Machine name already set");
            return Ok(());
        }

        let machine_name = Self::pull_machine_name(self.toml_config.client_config()).await?;
        self.toml_config.client_config_mut().set_machine_name(Some(machine_name));
        Ok(())
    }

    async fn pull_machine_name(client_config: &ClientConfig) -> anyhow::Result<String> {
        let client = Client::new();

        let res = client
            .get(format!(
                "{}/machines/machine-name-over-api-key",
                client_config.base_server_http_url()
            ))
            .header("X-Api-Key", client_config.api_key())
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => Ok(res.text().await?),
            status => Err(anyhow!("Failed to pull machine name: {}", status)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::PathBuf;
    use crate::config::app_config::AppConfig;

    #[ignore = "Needs config for this to work"]
    #[test]
    fn load_config() {
        let config_path = PathBuf::from(env::current_dir().unwrap());
        let config_path = config_path.parent().unwrap().parent().unwrap().join("observer.toml");
        let app_config = AppConfig::load_from_path(config_path).unwrap();
        println!("Path: {:#?}", app_config);
    }
}