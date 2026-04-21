use std::sync::OnceLock;
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use open_eye::collector::{
    speedtest::collector::{SpeedtestError, SpeedtestResult},
};

static LAST_METRICS: OnceLock<RwLock<Option<SpeedtestMetrics>>> = OnceLock::new();

fn last_metrics() -> &'static RwLock<Option<SpeedtestMetrics>> {
    LAST_METRICS.get_or_init(|| RwLock::new(None))
}

pub async fn get_last_metrics() -> Option<SpeedtestMetrics> {
    last_metrics().read().await.clone()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedtestMetrics {
    pub download_mbps: Option<f64>,
    pub upload_mbps: Option<f64>,
    pub ping_ms: Option<f64>,
}

impl SpeedtestMetrics {
    pub async fn collect() -> Result<SpeedtestMetrics, SpeedtestError> {
        let result = open_eye::collector::speedtest::collector::run().await?;

        let metrics = SpeedtestMetrics {
            download_mbps: Some(result.download_mbps.round()),
            upload_mbps: Some(result.upload_mbps.round()),
            ping_ms: Some(result.ping_ms.round()),
        };

        let mut last = last_metrics().write().await;
        *last = Some(metrics.clone());

        Ok(metrics)
    }

    pub async fn run() -> Result<(), SpeedtestError> {
        let metrics = Self::collect().await?;

        debug!("Speedtest metrics collected: {:?}", metrics);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_speedtest_metrics_collection() {
        let metrics = SpeedtestMetrics::collect().await.expect("Failed to collect speedtest metrics");
        println!("Collected Speedtest Metrics: {:?}", metrics);
        assert!(metrics.download_mbps.is_some());
        assert!(metrics.upload_mbps.is_some());
        assert!(metrics.ping_ms.is_some());
    }
}