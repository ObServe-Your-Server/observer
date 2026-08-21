use crate::config::config_parts::interval_config::IntervalConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use getset::Getters;
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