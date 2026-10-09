use crate::grpc::v1::metrics_tunnel_client::MetricsTunnelClient;
use crate::grpc::v1::response_builder::build_metrics_response;
use crate::storage_engine::storage_engine::StorageEngine;
use anyhow::{Result, anyhow};
use tokio::time::timeout;
use std::sync::Arc;
use std::time::Duration;
use rand::RngExt;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::{metadata::MetadataValue, transport::ClientTlsConfig};

pub struct MetricsTunnel {
    url: String,
    api_key: String,
    reconnect_budget: Duration,
    storage_engine: Arc<StorageEngine>,
}

impl MetricsTunnel {
    pub fn new(
        url: impl Into<String>,
        api_key: impl Into<String>,
        storage_engine: Arc<StorageEngine>,
    ) -> Self {
        Self {
            url: url.into(),
            api_key: api_key.into(),
            reconnect_budget: Duration::from_hours(24),
            storage_engine,
        }
    }

    pub async fn run_blocking(&self) -> Result<()> {
        let mut deadline = tokio::time::Instant::now() + self.reconnect_budget;

        loop {
            let connected_at = tokio::time::Instant::now();
            // check if the current time is over the deadline
            if connected_at >= deadline {
                return Err(anyhow!("GRPC client went over the time budget for reconnections and failed."));
            }

            // timeout with 15min. Then reconnect -> we had issues where the socket doesnt recognise a close
            // when connected long to one server
            match timeout(Duration::from_mins(15), self.connect_and_serve()).await {
                Ok(job_res) => {
                    match job_res {
                        Ok(_) => {
                            // shouldnt happen. For now gets treated as an error
                            log::error!("Metrics tunnel finished, which shouldn't happen");
                        }
                        Err(err) => {
                            // job failed and the deadline doesnt increase
                            log::error!("Metrics tunnel failed: {}", err);
                            // wait a random time
                            let secs = rand::rng().random_range(1..=30);
                            tokio::time::sleep(Duration::from_secs(secs)).await;
                        }
                    }
                },
                Err(err) => {
                    // job run for 1h so now reconnect. This is not an error
                    log::debug!("Job run for 15min. Now timeouted which is not an error.");

                    // all went good for the 1h so now increase the deadline
                    deadline = tokio::time::Instant::now() + self.reconnect_budget;
                    continue;
                },
            }
        }
    }

    async fn connect_and_serve(&self) -> Result<()> {
        // general gRPC channel
        let channel = self
            .connect_with_retries()
            .await
            .map_err(|e| tonic::Status::unavailable(e.to_string()))?;

        let api_key: MetadataValue<_> = match MetadataValue::try_from(self.api_key.to_string()) {
            Ok(key) => key,
            Err(err) => {
                log::error!("Invalid string as api key. Error: {}", err);
                return Err(anyhow!(err).context("Invalid string as api key."));
            }
        };

        // the server authenticates every call, so the interceptor adds the key to all of them
        let mut client = MetricsTunnelClient::with_interceptor(
            channel,
            move |mut request: tonic::Request<()>| {
                request.metadata_mut().insert("x-api-key", api_key.clone());
                Ok(request)
            },
        );

        log::info!("Established connection to grpc server");
        // now establish the base tunnel connection
        let mut request_stream = match client.tunnel(tonic::Request::new(())).await {
            Ok(r) => r.into_inner(),
            Err(err) => {
                log::error!("Received error in metrics tunnel: {}", err);
                return Err(anyhow!("Received error in metrics tunnel: {}", err));
            }
        };

        while let Some(request) = request_stream.next().await {
            match request {
                Ok(request) => {
                    let response =
                        build_metrics_response(self.storage_engine.clone(), request).await;
                    match client.metrics_response(response).await {
                        Ok(_) => {
                            log::debug!("Sent metrics response through the tunnel");
                        }
                        Err(err) => {
                            // a single failed response must not tear down the tunnel,
                            // if the connection is really broken the stream below ends with an error
                            log::error!("Failed to transmit metrics response: {}", err);
                        }
                    }
                }
                Err(err) => {
                    return Err(anyhow!("Metrics tunnel stream error, tunnel likely closed: {}", err))
                }
            }
        }
        Ok(())
    }

    async fn connect_with_retries(&self) -> Result<Channel> {
        let deadline = tokio::time::Instant::now() + self.reconnect_budget;
        let mut last_err = None;

        loop {
            // connect the socket to the given url
            match Self::connect_socket(&self.url).await {
                Ok(channel) => {
                    return Ok(channel);
                }
                Err(e) => {
                    log::error!(
                        "Error connecting to grpc server: {} with error: {}",
                        self.url,
                        e
                    );
                    last_err = Some(e);

                    if tokio::time::Instant::now() >= deadline {
                        log::error!(
                            "giving up reconnecting to {} after {:?}",
                            self.url,
                            self.reconnect_budget
                        );
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
        Err(last_err.unwrap())
    }

    async fn connect_socket(url: &str) -> Result<Channel> {
        let endpoint = tonic::transport::Channel::from_shared(url.to_string())?
            .keep_alive_while_idle(true)
            .http2_keep_alive_interval(Duration::from_secs(15))
            .keep_alive_timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(15))
            .buffer_size(256);

        // if the url starts with https then connect with tls
        let endpoint = if url.starts_with("https://") {
            endpoint.tls_config(ClientTlsConfig::new().with_native_roots())?
        } else {
            endpoint
        };

        Ok(endpoint.connect().await?)
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::init_logging;
    use std::time::SystemTime;
    use tokio::sync::mpsc;
    use tokio_stream::{wrappers::ReceiverStream, StreamExt};
    use tonic::transport::Server;
    use tonic::{metadata::MetadataValue, Request};

    const SERVER_URL: &str = "http://localhost:50051";

    #[ignore = "requires a grpc server running"]
    #[tokio::test]
    async fn test_base_transfer_with_server() {
        let channel = tonic::transport::Channel::from_static(SERVER_URL)
            .keep_alive_while_idle(true)
            .connect()
            .await
            .expect("failed to connect to server");

        let mut client = MetricsTunnelClient::new(channel);

        let (resp_tx, resp_rx) = mpsc::channel(4);
        let outbound = ReceiverStream::new(resp_rx);

        let mut request = Request::new(outbound);
        request
            .metadata_mut()
            .insert("api_key", MetadataValue::try_from("test-key").unwrap());

        let mut inbound = client
            .base_transfer(request)
            .await
            .expect("base_transfer call failed")
            .into_inner();

        let request_data = inbound
            .next()
            .await
            .expect("server closed stream without sending")
            .expect("stream error");

        println!("Received RequestData: id={:?}", request_data.request_id);

        resp_tx
            .send(MetricsResponse {
                request_id: request_data.request_id.clone(),
                returned_metric: None,
            })
            .await
            .expect("failed to send response");

        tokio::time::sleep(Duration::from_secs(1)).await;
        println!(
            "Sent MetricsResponse for request_id={}",
            request_data.request_id
        );
    }
}
*/
