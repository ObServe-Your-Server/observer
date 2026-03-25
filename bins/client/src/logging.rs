use std::env;

pub enum LogTarget {
    MetricCollection,
    Speedtest,
    CommandPolling,
    DockerSystem,
    HostSystemHealth,
}

impl LogTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogTarget::MetricCollection => "metric_collection",
            LogTarget::Speedtest => "speedtest",
            LogTarget::CommandPolling => "command_polling",
            LogTarget::DockerSystem => "docker_system",
            LogTarget::HostSystemHealth => "host_system_health",
        }
    }
}

pub fn init_logging() {
    let level_str = env::var("OBSERVER_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let level = level_str
        .parse::<log::LevelFilter>()
        .unwrap_or(log::LevelFilter::Info);

    env_logger::Builder::new()
        .filter(Some("observer"), level)
        .init();

    log::debug!("Logging initialized at level: {}", level);
}
