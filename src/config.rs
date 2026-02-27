use std::fs;
use std::sync::OnceLock;
use log::debug;
use serde::Deserialize;

#[derive(Debug)]
pub struct Config {
    pub version: &'static str,
    pub server: ServerConfig,
    pub intervals: IntervalsConfig,
}

#[derive(Debug, Deserialize)]
struct TomlConfig {
    server: ServerConfig,
    intervals: IntervalsConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub base_metrics_url: String,
    pub base_commands_url: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct IntervalsConfig {
    pub metric_secs: u16,
    pub command_poll_secs: u16,
    pub speedtest_secs: u32,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Config not initialized - call init_config() first")
}

pub fn init_config(path: &str) -> Result<&'static Config, String> {
    let config = load_config(path)?;
    Ok(CONFIG.get_or_init(|| config))
}

fn load_config(path: &str) -> Result<Config, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

    let toml: TomlConfig = toml::from_str(&raw)
        .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?;

    let config = Config {
        version: env!("CARGO_PKG_VERSION"),
        server: toml.server,
        intervals: toml.intervals,
    };

    debug!("Config loaded from '{}'", path);
    validate(&config)?;
    debug!("Config: {:?}", config);

    Ok(config)
}

fn validate(c: &Config) -> Result<(), String> {
    let i = &c.intervals;

    if !(1..=60).contains(&i.metric_secs) {
        return Err(format!(
            "intervals.metric_secs must be 1–60, got {}",
            i.metric_secs
        ));
    }
    if !(1..=60).contains(&i.command_poll_secs) {
        return Err(format!(
            "intervals.command_poll_secs must be 1–60, got {}",
            i.command_poll_secs
        ));
    }
    if !(60..=86400).contains(&i.speedtest_secs) {
        return Err(format!(
            "intervals.speedtest_secs must be 60–86400, got {}",
            i.speedtest_secs
        ));
    }

    Ok(())
}
