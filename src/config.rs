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
    pub base_url: String,
    pub api_key: String,
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

    let mode_str = env::var("OBSERVER_MODE").unwrap_or_else(|_| Mode::ALL_IN_ONE.to_string());
    debug!("Mode: {}", mode_str);

    match mode_str.to_lowercase().as_str() {
        Mode::CLIENT => {
            let base_url = env::var("OBSERVER_BASE_URL")
                .map_err(|_| "OBSERVER_BASE_URL is required in client mode".to_string())?;
            let api_key = env::var("OBSERVER_API_KEY")
                .map_err(|_| "OBSERVER_API_KEY is required in client mode".to_string())?;
            debug!("Client config: base_url={}", base_url);

            Ok(Config {
                version,
                mode: Mode::Client,
                client_config: Some(ClientConfig { base_url, api_key }),
                all_in_one_config: None,
            })
        }
        Mode::ALL_IN_ONE => {
            let port = env::var("OBSERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
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
