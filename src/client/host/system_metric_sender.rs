use log::{debug, error, info};
use reqwest::Client;

use super::system_metric_collection::Metrics;
use crate::{client::metric_collection_errors::CollectionError, config::get_config};

pub async fn send(client: &Client, metrics: &Metrics) -> Result<(), CollectionError> {
    let config = get_config();

    debug!("Payload to send: {:?}", metrics);

    let result = client
        .post(&config.server.base_metrics_url)
        .header("X-Api-Key", &config.server.api_key)
        .json(metrics)
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
        Ok(resp) => {
            Err(CollectionError::ServerRejected(resp.status()))
        }
        Err(e) => {
            Err(CollectionError::SendFailed(e))
        }
    }
}
