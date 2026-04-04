use log::{debug, info};
use reqwest::Client;

use crate::{config::get_config, mapper::host_metrics_models::mapped_host_system_metrics::MappedHostSystemMetrics, scheduling::collection_error::CollectionError};

pub struct HostSystemMetricsSender{}

impl HostSystemMetricsSender {
    pub async fn send(mapped_host_system_metrics: MappedHostSystemMetrics) -> Result<(), CollectionError>{
        let config = get_config();
    
        debug!("Payload to send: {:?}", mapped_host_system_metrics);
    
        let result = Client::new()
            .post(&config.server.base_metrics_url)
            .header("X-Api-Key", &config.server.api_key)
            .json(&mapped_host_system_metrics)
            .send()
            .await;
    
        match result {
            Ok(resp) if resp.status().is_success() => {
                info!(
                    "Metrics sent ({}) http version: {:?}",
                    resp.status(),
                    resp.version()
                );
                Ok(())
            }
            Ok(resp) => Err(CollectionError::ServerRejected(resp.status())),
            Err(e) => Err(CollectionError::SendFailed(e)),
        }
    }
}