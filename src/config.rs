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

    if !(2..=60).contains(&i.metric_secs) {
        return Err(format!(
            "intervals.metric_secs must be 2–60, got {}",
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn valid_toml() -> &'static str {
        r#"
[server]
base_metrics_url  = "http://localhost:8080/v1/ingest"
base_commands_url = "http://localhost:8080/api/commands"
api_key           = "test-key"

[intervals]
metric_secs       = 5
command_poll_secs = 10
speedtest_secs    = 3600
"#
    }

    fn write_temp(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn test_load_valid_config() {
        let f = write_temp(valid_toml());
        let config = load_config(f.path().to_str().unwrap()).unwrap();
        assert_eq!(config.server.base_metrics_url, "http://localhost:8080/v1/ingest");
        assert_eq!(config.intervals.metric_secs, 5);
        assert_eq!(config.intervals.speedtest_secs, 3600);
    }

    #[test]
    fn test_missing_file_returns_error() {
        let result = load_config("/nonexistent/path/config.toml");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read config file"));
    }

    #[test]
    fn test_invalid_toml_returns_error() {
        let f = write_temp("this is not valid toml ][");
        let result = load_config(f.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse config file"));
    }

    #[test]
    fn test_metric_secs_too_low() {
        let f = write_temp(valid_toml().replace("metric_secs       = 5", "metric_secs = 1").as_str());
        let result = load_config(f.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("metric_secs"));
    }

    #[test]
    fn test_metric_secs_too_high() {
        let f = write_temp(valid_toml().replace("metric_secs       = 5", "metric_secs = 61").as_str());
        let result = load_config(f.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("metric_secs"));
    }

    #[test]
    fn test_command_poll_secs_out_of_range() {
        let f = write_temp(valid_toml().replace("command_poll_secs = 10", "command_poll_secs = 0").as_str());
        let result = load_config(f.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("command_poll_secs"));
    }

    #[test]
    fn test_speedtest_secs_too_low() {
        let f = write_temp(valid_toml().replace("speedtest_secs    = 3600", "speedtest_secs = 59").as_str());
        let result = load_config(f.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("speedtest_secs"));
    }

    #[test]
    fn test_speedtest_secs_too_high() {
        let f = write_temp(valid_toml().replace("speedtest_secs    = 3600", "speedtest_secs = 86401").as_str());
        let result = load_config(f.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("speedtest_secs"));
    }
}
