use crate::grpc::v1::response_builder::build_metrics_response;
use crate::storage_engine::storage_engine::StorageEngine;
use anyhow::{Result, anyhow};
use chrono::{DateTime, TimeZone, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::{metadata::MetadataValue, transport::ClientTlsConfig};
use crate::grpc::v1::metrics_request::Query;
use crate::grpc::v1::metrics_tunnel_client::MetricsTunnelClient;
use crate::grpc::v1::MetricsRequest;

/// Which slice of history a request wants: an inclusive `[start, end]` time
/// range, or just the most recent `n` entries.
#[derive(Clone, Copy, Debug)]
pub enum QueryRange {
    Between(DateTime<Utc>, DateTime<Utc>),
    LastN(u64),
}

impl QueryRange {
    fn from_request(request: &MetricsRequest) -> Self {
        match &request.query {
            Some(Query::LastN(n)) => QueryRange::LastN((*n).max(0) as u64),
            Some(Query::Range(range)) => {
                let start = Utc
                    .timestamp_opt(range.start, 0)
                    .single()
                    .unwrap_or_else(Utc::now);
                let end = Utc
                    .timestamp_opt(range.end, 0)
                    .single()
                    .unwrap_or_else(Utc::now);
                QueryRange::Between(start, end)
            }
            None => QueryRange::Between(Utc::now(), Utc::now()),
        }
    }
}

pub struct MetricsTunnel {
    url: String,
    api_key: String,
    reconnect_budget: Duration,
    storage_engine: Arc<StorageEngine>,
}

impl MetricsTunnel {
    pub fn new(url: String, api_key: String, storage_engine: Arc<StorageEngine>) -> Self {
        Self {
            url,
            api_key,
            reconnect_budget: Duration::from_hours(24),
            storage_engine,
        }
    }

    /// Runs the tunnel until it is closed, reconnecting whenever the connection
    /// drops. Every time a (re)connect is needed, retries happen for up to
    /// `reconnect_budget` before giving up entirely and returning an error.
    /// Once a connection is (re)established, the budget resets for the next drop.
    ///
    /// This covers both failures to (re)connect the socket (handled inside
    /// `connect_and_serve`/`connect_with_retries`) and connections that are
    /// established but immediately fail at the application level (e.g. a proxy
    /// 403 surfacing as bad stream framing) — without a deadline here, that
    /// second case would reconnect instantly in a tight loop.
    pub async fn run_blocking(&self) -> Result<()> {
        let mut deadline = tokio::time::Instant::now() + self.reconnect_budget;

        loop {
            let connected_at = tokio::time::Instant::now();
            self.connect_and_serve().await?;
            log::warn!("metrics tunnel connection lost, reconnecting");

            // a connection that survived a while is a sign the server is healthy;
            // give the next drop a fresh budget instead of accumulating downtime.
            if connected_at >= deadline {
                deadline = tokio::time::Instant::now() + self.reconnect_budget;
            }

            if tokio::time::Instant::now() >= deadline {
                return Err(anyhow!(format!(
                    "Tunnel kept failing for over {}",
                    self.reconnect_budget.as_secs()
                )));
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn connect_and_serve(&self) -> Result<()> {
        // general gRPC channel
        let channel = self
            .connect_with_retries()
            .await
            .map_err(|e| tonic::Status::unavailable(e.to_string()))?;

        let mut client = MetricsTunnelClient::new(channel);

        // now establish the base tunnel connection
        // first create a receiver stream and the initial connection request to init the stream

        let mut initial_request = tonic::Request::new(());
        let api_key = match MetadataValue::try_from(self.api_key.to_string()) {
            Ok(key) => key,
            Err(err) => {
                // TODO error implementation
                log::error!("Invalid string as api key. Error: {}", err);
                return Err(anyhow!(err).context("Invalid string as api key."));
            }
        };
        initial_request.metadata_mut().insert("x-api-key", api_key);

        let mut request_stream = match client.tunnel(initial_request).await {
            Ok(r) => r.into_inner(),
            Err(err) => {
                // TODO error implementation
                log::error!("Received error in metrics tunnel: {}", err);
                return Err(anyhow!("TODO"));
            }
        };

        while let Some(request) = request_stream.next().await {
            match request {
                Ok(request) => {
                    let response = build_metrics_response(request).await;
                    match client.metrics_response(response).await {
                        Ok(_) => {
                            // TODO log that all went okay
                        }
                        Err(_err) => {
                            // Error handling of transmitting
                        }
                    }
                }
                Err(_err) => {
                    // TODO error implementation
                    // maybe reconnect etc
                    todo!()
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
                    log::info!("Connected to the server at {}", self.url);
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
