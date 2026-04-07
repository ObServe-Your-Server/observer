use std::env;
use log::debug;

// the init should be done either from a top level crate and
// then the value for the logging configuration should pe set.
// It is also possible to run this in a test and have logging there
pub fn init_logging() {
    let level = env::var("OPEN_EYE_LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .parse::<log::LevelFilter>()
        .unwrap_or(log::LevelFilter::Info);

    let _ = env_logger::Builder::new()
        .filter(None, log::LevelFilter::Off) // silence all other crates
        .filter(Some("open_eye"), level) // only open_eye crate
        .try_init(); // don't panic if already initialized

    debug!("Logging initialized at level: {}", level);
}
