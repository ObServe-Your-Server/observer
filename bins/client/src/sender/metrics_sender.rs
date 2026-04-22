use std::any::type_name;
use crate::{config::get_config, scheduling::collection_error::CollectionError};
use log::{debug, info};
use reqwest::Client;
use serde::Serialize;
use std::fmt::Debug;

pub struct MetricsSender {}

impl MetricsSender {
    pub async fn send<T>(payload: T, metrics_url: String) -> Result<(), CollectionError>
    where
        T: Debug + Serialize,
    {
        let config = get_config();

        debug!("[{}] Payload to send: {:?}", type_name::<T>(), payload);

        let result = Client::new()
            .post(&metrics_url)
            .header("X-Api-Key", &config.server.api_key)
            .json(&payload)
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                info!(
                    "[{}] Metrics sent ({}) http version: {:?}",
                    type_name::<T>(),
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
