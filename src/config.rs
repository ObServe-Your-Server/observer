use std::env;
use std::sync::OnceLock;
use log::debug;

#[derive(Debug)]
pub enum Mode {
    Client,
    AllInOne,
}

impl Mode {
    const CLIENT: &'static str = "client";
    const ALL_IN_ONE: &'static str = "all-in-one";
}

#[derive(Debug)]
pub struct ClientConfig {
    pub base_metrics_url: String,
    pub base_commands_url: String,
    pub api_key: String,
    pub active_streaming_interval_secs: u16,
    pub inactive_streaming_interval_secs: u16,
    pub command_poll_interval_secs: u16,
}

#[derive(Debug)]
pub struct AllInOneConfig {
    pub port: u16,
}

#[derive(Debug)]
pub struct Config {
    pub version: String,
    pub mode: Mode,
    pub client_config: Option<ClientConfig>,
    pub all_in_one_config: Option<AllInOneConfig>,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Config not initialized — call init_config() first")
}

pub fn init_config() -> Result<&'static Config, String> {
    let config = load_config()?;
    Ok(CONFIG.get_or_init(|| config))
}

fn load_config() -> Result<Config, String> {
    // Load .env file if present, ignore if missing
    let _ = dotenvy::dotenv();

    let version = env!("CARGO_PKG_VERSION").to_string();
    debug!("Loading config, version={}", version);

    let mode_str = env::var("OBSERVER_MODE")
        .map_err(|_| "OBSERVER_MODE is required".to_string())?;
    debug!("Mode: {}", mode_str);

    match mode_str.to_lowercase().as_str() {
        Mode::CLIENT => {
            let base_metrics_url = env::var("OBSERVER_BASE_METRICS_URL")
                .map_err(|_| "OBSERVER_BASE_METRICS_URL is required in client mode".to_string())?;
            let base_commands_url = env::var("OBSERVER_BASE_COMMANDS_URL")
                .map_err(|_| "OBSERVER_BASE_COMMANDS_URL is required in client mode".to_string())?;
            let api_key = env::var("OBSERVER_API_KEY")
                .map_err(|_| "OBSERVER_API_KEY is required in client mode".to_string())?;
            // Valid range: 1–15 seconds. Change the constants below to adjust.
            const ACTIVE_MIN: u16 = 1;
            const ACTIVE_MAX: u16 = 15;
            let active_streaming_interval_secs = env::var("OBSERVER_ACTIVE_STREAMING_INTERVAL_SECS")
                .map_err(|_| "OBSERVER_ACTIVE_STREAMING_INTERVAL_SECS is required".to_string())?
                .parse::<u16>()
                .map_err(|_| "OBSERVER_ACTIVE_STREAMING_INTERVAL_SECS must be a valid number".to_string())?;
            if active_streaming_interval_secs < ACTIVE_MIN || active_streaming_interval_secs > ACTIVE_MAX {
                return Err(format!(
                    "OBSERVER_ACTIVE_STREAMING_INTERVAL_SECS must be between {} and {} seconds, got {}",
                    ACTIVE_MIN, ACTIVE_MAX, active_streaming_interval_secs
                ));
            }

            // Valid range: 10–300 seconds. Change the constants below to adjust.
            const INACTIVE_MIN: u16 = 10;
            const INACTIVE_MAX: u16 = 300;
            let inactive_streaming_interval_secs = env::var("OBSERVER_INACTIVE_STREAMING_INTERVAL_SECS")
                .map_err(|_| "OBSERVER_INACTIVE_STREAMING_INTERVAL_SECS is required".to_string())?
                .parse::<u16>()
                .map_err(|_| "OBSERVER_INACTIVE_STREAMING_INTERVAL_SECS must be a valid number".to_string())?;
            if inactive_streaming_interval_secs < INACTIVE_MIN || inactive_streaming_interval_secs > INACTIVE_MAX {
                return Err(format!(
                    "OBSERVER_INACTIVE_STREAMING_INTERVAL_SECS must be between {} and {} seconds, got {}",
                    INACTIVE_MIN, INACTIVE_MAX, inactive_streaming_interval_secs
                ));
            }
            // Valid range: 1–60 seconds. Change the constants below to adjust.
            const COMMAND_POLL_MIN: u16 = 1;
            const COMMAND_POLL_MAX: u16 = 60;
            let command_poll_interval_secs = env::var("OBSERVER_COMMAND_POLL_INTERVAL_SECS")
                .map_err(|_| "OBSERVER_COMMAND_POLL_INTERVAL_SECS is required".to_string())?
                .parse::<u16>()
                .map_err(|_| "OBSERVER_COMMAND_POLL_INTERVAL_SECS must be a valid number".to_string())?;
            if command_poll_interval_secs < COMMAND_POLL_MIN || command_poll_interval_secs > COMMAND_POLL_MAX {
                return Err(format!(
                    "OBSERVER_COMMAND_POLL_INTERVAL_SECS must be between {} and {} seconds, got {}",
                    COMMAND_POLL_MIN, COMMAND_POLL_MAX, command_poll_interval_secs
                ));
            }

            debug!("Client config: base_metrics_url={} base_commands_url={} active_interval={}s inactive_interval={}s command_poll={}s",
                base_metrics_url, base_commands_url, active_streaming_interval_secs, inactive_streaming_interval_secs, command_poll_interval_secs);

            Ok(Config {
                version,
                mode: Mode::Client,
                client_config: Some(ClientConfig {
                    base_metrics_url,
                    base_commands_url,
                    api_key,
                    active_streaming_interval_secs,
                    inactive_streaming_interval_secs,
                    command_poll_interval_secs,
                }),
                all_in_one_config: None,
            })
        }
        Mode::ALL_IN_ONE => {
            let port = env::var("OBSERVER_PORT")
                .map_err(|_| "OBSERVER_PORT is required".to_string())?
                .parse::<u16>()
                .map_err(|_| "OBSERVER_PORT must be a valid port number (0–65535)".to_string())?;
            debug!("All-in-one config: port={}", port);

            Ok(Config {
                version,
                mode: Mode::AllInOne,
                client_config: None,
                all_in_one_config: Some(AllInOneConfig { port }),
            })
        }
        other => Err(format!(
            "Unknown OBSERVER_MODE: '{}'. Expected '{}' or '{}'",
            other,
            Mode::CLIENT,
            Mode::ALL_IN_ONE
        )),
    }
}

