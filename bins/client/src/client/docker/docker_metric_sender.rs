use log::{debug, error, info};
use reqwest::Client;

use super::collector::ContainerStats;
use crate::{client::metric_collection_errors::CollectionError, config::get_config};

pub async fn send(client: &Client, containers: &[ContainerStats]) -> Result<(), CollectionError> {
    let config = get_config();
    let url = &config.server.base_docker_url;

    debug!("Docker payload to send: {:?}", containers);

    let result = client
        .post(url.as_str())
        .header("X-Api-Key", &config.server.api_key)
        .json(containers)
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            info!("Docker metrics sent ({})", resp.status());
            Ok(())
        }
        Ok(resp) => Err(CollectionError::ServerRejected(resp.status())),
        Err(e) => Err(CollectionError::SendFailed(e)),
    }
}
