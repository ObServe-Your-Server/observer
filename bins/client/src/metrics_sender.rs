use std::fmt;
/*

use log::{debug, info};
use reqwest::Client;
use serde::Serialize;

use crate::{client::metric_collection_errors::CollectionErrorOld, config::get_config};

pub async fn send<T>(client: &Client, metrics: T) -> Result<(), CollectionErrorOld>
where
    T: Serialize + fmt::Debug,
{
    let config = get_config();

    debug!("Payload to send: {:?}", metrics);

    let result = client
        .post(&config.server.base_metrics_url)
        .header("X-Api-Key", &config.server.api_key)
        .json(&metrics)
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
        Ok(resp) => Err(CollectionErrorOld::ServerRejected(resp.status())),
        Err(e) => Err(CollectionErrorOld::SendFailed(e)),
    }
}*/
