use crate::grpc::v1::{FullRequest, MetricsRequest, MetricsResponse};
use crate::grpc::v1::metrics_request::RequestedMetric as RequestKind;
use crate::grpc::v1::metrics_request::Query as MetricsRequestQuery;
use crate::grpc::v1::metrics::container_runtime_request::MessageType as ContainerRequestKind;
use crate::grpc::v1::metrics_tunnel_client::MetricsTunnelClient;
use crate::grpc::v1::response_builder::{
    build_container_runtime_response, build_cpu_response, build_disk_response,
    build_full_response, build_memory_response, build_network_response, build_process_response,
    build_speedtest_response, build_system_response,
};

use crate::storage_engine::storage_engine::StorageEngine;
use chrono::{DateTime, TimeZone, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::{metadata::MetadataValue, transport::ClientTlsConfig, Request};

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
            Some(MetricsRequestQuery::LastN(n)) => QueryRange::LastN((*n).max(0) as u64),
            Some(MetricsRequestQuery::Range(range)) => {
                let start = Utc.timestamp_opt(range.start, 0).single().unwrap_or_else(Utc::now);
                let end = Utc.timestamp_opt(range.end, 0).single().unwrap_or_else(Utc::now);
                QueryRange::Between(start, end)
            }
            None => QueryRange::Between(Utc::now(), Utc::now()),
        }
    }
}

/// Dispatches `request` to the builder for the metric it names. A request that
/// names none is treated as asking for everything. Build failures are logged
/// and answered with an unset `returned_metric` rather than dropping the connection.
async fn build_response(request: &MetricsRequest, storage_engine: Arc<StorageEngine>) -> MetricsResponse {
    let fallback = RequestKind::FullRequest(FullRequest {});
    let requested_metric = request.requested_metric.as_ref().unwrap_or(&fallback);
    let range = QueryRange::from_request(request);

    let built = match requested_metric {
        RequestKind::CpuRequest(_) => build_cpu_response(range, storage_engine).await,
        RequestKind::MemoryRequest(_) => build_memory_response(range, storage_engine).await,
        RequestKind::DiskRequest(_) => build_disk_response(range, storage_engine).await,
        RequestKind::NetworkRequest(_) => build_network_response(range, storage_engine).await,
        RequestKind::SystemRequest(_) => build_system_response(range, storage_engine).await,
        RequestKind::SpeedtestRequest(_) => build_speedtest_response(range, storage_engine).await,
        RequestKind::ProcessRequest(req) => {
            build_process_response(range, req.number_of_processes, storage_engine).await
        }
        RequestKind::ContainerRuntimeRequest(req) => {
            // an unset message_type means "every run", same as GetFullMetrics
            let container_id = match req.message_type.as_ref() {
                Some(ContainerRequestKind::OneContainerMetrics(one)) => Some(one.container_id.as_str()),
                Some(ContainerRequestKind::FullMetrics(_)) | None => None,
            };
            build_container_runtime_response(range, container_id, storage_engine).await
        }
        RequestKind::FullRequest(_) => build_full_response(range, storage_engine).await,
    };

    let returned_metric = match built {
        Ok(metric) => Some(metric),
        Err(e) => {
            log::error!("failed to build response for request {}: {e}", request.request_id);
            None
        }
    };

    MetricsResponse {
        request_id: request.request_id.clone(),
        returned_metric,
    }
}

pub struct MetricsTunnel {
    url: &'static str,
    api_key: String,
    reconnect_budget: Duration,
    storage_engine: Arc<StorageEngine>,
}

impl MetricsTunnel {
    pub fn new(url: &'static str, api_key: String, storage_engine: Arc<StorageEngine>) -> Self {
        Self {
            url,
            api_key,
            reconnect_budget: Duration::from_secs(5 * 60),
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
    pub async fn run_blocking(&self) -> Result<(), tonic::Status> {
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
                return Err(tonic::Status::unavailable(format!(
                    "metrics tunnel kept failing for over {:?}, giving up",
                    self.reconnect_budget
                )));
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn connect_and_serve(&self) -> Result<(), tonic::Status> {
        // (re)connect our channel, retrying within the reconnect budget
        let channel = self
            .connect_with_retries()
            .await
            .map_err(|e| tonic::Status::unavailable(e.to_string()))?;

        let mut client = MetricsTunnelClient::new(channel);

        // creates the tx and rx for the metrics responses we send back to the server
        let (tx, rx) = mpsc::channel::<MetricsResponse>(16);
        // wraps it into a stream to hand to the server
        let outbound = ReceiverStream::new(rx);

        // creates the request with the api key
        let mut request = Request::new(outbound);
        let api_key = match MetadataValue::try_from(self.api_key.as_str()) {
            Ok(v) => v,
            Err(e) => {
                log::error!("invalid api_key: {e}");
                return Ok(());
            }
        };
        request.metadata_mut().insert("x-api-key", api_key);

        // open the bidi stream — server sends MetricsRequest, we send MetricsResponse back.
        // Non-retryable statuses (bad/expired api key etc.) are propagated so the
        // caller stops instead of hammering the server in a tight reconnect loop;
        // everything else is treated as a transient connect failure.
        let response = match client.base_transfer(request).await {
            Ok(r) => r,
            Err(e) => {
                log::error!("base_transfer call failed: {e}");
                match e.code() {
                    tonic::Code::Unauthenticated | tonic::Code::PermissionDenied | tonic::Code::InvalidArgument => {
                        return Err(e);
                    }
                    _ => return Ok(()),
                }
            }
        };
        // get the inner stream to receive incoming requests from the server
        let mut inbound = response.into_inner();

        while let Some(result) = inbound.next().await {
            match result {
                Ok(req_data) => {
                    log::debug!("received request: {:?}", req_data);
                    let response = build_response(&req_data, self.storage_engine.clone()).await;
                    if tx.send(response).await.is_err() {
                        log::error!("response channel closed");
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Error receiving metrics request: {}", e);
                    break;
                }
            }
        }

        // stream ended (cleanly closed or errored) — caller will reconnect and restart
        Ok(())
    }

    /// Retries connecting until it succeeds or `reconnect_budget` elapses since
    /// the first attempt, whichever comes first.
    async fn connect_with_retries(&self) -> Result<Channel, tonic::transport::Error> {
        let deadline = tokio::time::Instant::now() + self.reconnect_budget;
        let mut last_err = None;

        loop {
            // connect the socket to the given url
            match Self::connect_socket(self.url).await {
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

    async fn connect_socket(url: &'static str) -> Result<Channel, tonic::transport::Error> {
        let endpoint = tonic::transport::Channel::from_static(url)
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

        endpoint.connect().await
    }
}

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
