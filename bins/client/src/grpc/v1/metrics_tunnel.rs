use crate::grpc::v1::metrics_mapping;
use crate::grpc::v1::metrics_request::Query as MetricsRequestQuery;
use crate::grpc::v1::metrics_response::Response as MetricsResponseKind;
use crate::grpc::v1::metrics_tunnel_client::MetricsTunnelClient;
use crate::grpc::v1::{
    ContainerRuntimeStatsList, CpuMetricsList, DiskMetricsList, FullMetrics, MemoryMetricsList,
    MetricsRequest, MetricsResponse, MetricsType, NetworkMetricsList, ProcessesStatsList,
    SpeedtestMetricsList, SystemMetricsList,
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
enum QueryRange {
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

/// Builds the `MetricsResponse` for `request` by querying `storage_engine`
/// according to `request.query` (a time range or "last n entries") and
/// mapping the rows onto the matching proto list type. Query failures are
/// logged and result in an empty `response` rather than dropping the connection.
async fn build_response(request: &MetricsRequest, storage_engine: &StorageEngine) -> MetricsResponse {
    let request_type = MetricsType::try_from(request.r#type).unwrap_or(MetricsType::Full);
    let range = QueryRange::from_request(request);

    let response = match request_type {
        MetricsType::Cpu => {
            let rows = match &range {
                QueryRange::Between(start, end) => storage_engine.get_cpu_stats_between(*start, *end).await,
                QueryRange::LastN(n) => storage_engine.get_cpu_stats_latest(*n).await,
            };
            match rows {
                Ok(rows) => Some(MetricsResponseKind::CpuMetrics(CpuMetricsList {
                    items: rows.into_iter().map(metrics_mapping::cpu_metrics).collect(),
                })),
                Err(e) => {
                    log::error!("failed to query cpu stats: {e}");
                    None
                }
            }
        }
        MetricsType::Memory => {
            let rows = match &range {
                QueryRange::Between(start, end) => storage_engine.get_memory_stats_between(*start, *end).await,
                QueryRange::LastN(n) => storage_engine.get_memory_stats_latest(*n).await,
            };
            match rows {
                Ok(rows) => Some(MetricsResponseKind::MemoryMetrics(MemoryMetricsList {
                    items: rows.into_iter().map(metrics_mapping::memory_metrics).collect(),
                })),
                Err(e) => {
                    log::error!("failed to query memory stats: {e}");
                    None
                }
            }
        }
        MetricsType::Disk => {
            let rows = match &range {
                QueryRange::Between(start, end) => storage_engine.get_disk_stats_between(*start, *end).await,
                QueryRange::LastN(n) => storage_engine.get_disk_stats_latest(*n).await,
            };
            match rows {
                Ok(rows) => Some(MetricsResponseKind::DiskMetrics(DiskMetricsList {
                    items: rows.into_iter().map(metrics_mapping::disk_metrics).collect(),
                })),
                Err(e) => {
                    log::error!("failed to query disk stats: {e}");
                    None
                }
            }
        }
        MetricsType::Network => {
            let rows = match &range {
                QueryRange::Between(start, end) => storage_engine.get_network_stats_between(*start, *end).await,
                QueryRange::LastN(n) => storage_engine.get_network_stats_latest(*n).await,
            };
            match rows {
                Ok(rows) => Some(MetricsResponseKind::NetworkMetrics(NetworkMetricsList {
                    items: rows.into_iter().map(metrics_mapping::network_metrics).collect(),
                })),
                Err(e) => {
                    log::error!("failed to query network stats: {e}");
                    None
                }
            }
        }
        MetricsType::System => {
            let rows = match &range {
                QueryRange::Between(start, end) => storage_engine.get_system_stats_between(*start, *end).await,
                QueryRange::LastN(n) => storage_engine.get_system_stats_latest(*n).await,
            };
            match rows {
                Ok(rows) => Some(MetricsResponseKind::SystemMetrics(SystemMetricsList {
                    items: rows.into_iter().map(metrics_mapping::system_metrics).collect(),
                })),
                Err(e) => {
                    log::error!("failed to query system stats: {e}");
                    None
                }
            }
        }
        MetricsType::Process => {
            let rows = match &range {
                QueryRange::Between(start, end) => storage_engine.get_processes_stats_between(*start, *end).await,
                QueryRange::LastN(n) => storage_engine.get_processes_stats_latest(*n).await,
            };
            match rows {
                Ok(rows) => Some(MetricsResponseKind::ProcessMetrics(ProcessesStatsList {
                    items: rows.into_iter().map(metrics_mapping::processes_stats).collect(),
                })),
                Err(e) => {
                    log::error!("failed to query process stats: {e}");
                    None
                }
            }
        }
        MetricsType::Docker => {
            let rows = match &range {
                QueryRange::Between(start, end) => {
                    storage_engine.get_container_runtime_stats_between(*start, *end).await
                }
                QueryRange::LastN(n) => storage_engine.get_container_runtime_stats_latest(*n).await,
            };
            match rows {
                Ok(rows) => Some(MetricsResponseKind::ContainerMetrics(ContainerRuntimeStatsList {
                    items: rows.into_iter().map(metrics_mapping::container_runtime_stats).collect(),
                })),
                Err(e) => {
                    log::error!("failed to query container stats: {e}");
                    None
                }
            }
        }
        MetricsType::Speedtest => {
            let rows = match &range {
                QueryRange::Between(start, end) => storage_engine.get_speedtest_stats_between(*start, *end).await,
                QueryRange::LastN(n) => storage_engine.get_speedtest_stats_latest(*n).await,
            };
            match rows {
                Ok(rows) => Some(MetricsResponseKind::SpeedtestMetrics(SpeedtestMetricsList {
                    items: rows.into_iter().map(metrics_mapping::speedtest_metrics).collect(),
                })),
                Err(e) => {
                    log::error!("failed to query speedtest stats: {e}");
                    None
                }
            }
        }
        MetricsType::Full => {
            let (cpu, memory, disks, network, system, processes, containers, speedtest) = match &range {
                QueryRange::Between(start, end) => {
                    tokio::join!(
                        storage_engine.get_cpu_stats_between(*start, *end),
                        storage_engine.get_memory_stats_between(*start, *end),
                        storage_engine.get_disk_stats_between(*start, *end),
                        storage_engine.get_network_stats_between(*start, *end),
                        storage_engine.get_system_stats_between(*start, *end),
                        storage_engine.get_processes_stats_between(*start, *end),
                        storage_engine.get_container_runtime_stats_between(*start, *end),
                        storage_engine.get_speedtest_stats_between(*start, *end),
                    )
                }
                QueryRange::LastN(n) => {
                    tokio::join!(
                        storage_engine.get_cpu_stats_latest(*n),
                        storage_engine.get_memory_stats_latest(*n),
                        storage_engine.get_disk_stats_latest(*n),
                        storage_engine.get_network_stats_latest(*n),
                        storage_engine.get_system_stats_latest(*n),
                        storage_engine.get_processes_stats_latest(*n),
                        storage_engine.get_container_runtime_stats_latest(*n),
                        storage_engine.get_speedtest_stats_latest(*n),
                    )
                }
            };

            Some(MetricsResponseKind::FullMetrics(FullMetrics {
                cpu_metrics: Some(CpuMetricsList {
                    items: cpu.unwrap_or_default().into_iter().map(metrics_mapping::cpu_metrics).collect(),
                }),
                memory_metrics: Some(MemoryMetricsList {
                    items: memory.unwrap_or_default().into_iter().map(metrics_mapping::memory_metrics).collect(),
                }),
                disk_metrics: Some(DiskMetricsList {
                    items: disks.unwrap_or_default().into_iter().map(metrics_mapping::disk_metrics).collect(),
                }),
                network_metrics: Some(NetworkMetricsList {
                    items: network.unwrap_or_default().into_iter().map(metrics_mapping::network_metrics).collect(),
                }),
                system_metrics: Some(SystemMetricsList {
                    items: system.unwrap_or_default().into_iter().map(metrics_mapping::system_metrics).collect(),
                }),
                process_metrics: Some(ProcessesStatsList {
                    items: processes.unwrap_or_default().into_iter().map(metrics_mapping::processes_stats).collect(),
                }),
                container_metrics: Some(ContainerRuntimeStatsList {
                    items: containers.unwrap_or_default().into_iter().map(metrics_mapping::container_runtime_stats).collect(),
                }),
                speedtest_metrics: Some(SpeedtestMetricsList {
                    items: speedtest.unwrap_or_default().into_iter().map(metrics_mapping::speedtest_metrics).collect(),
                }),
            }))
        }
    };

    MetricsResponse {
        request_id: request.request_id.clone(),
        response,
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
    pub async fn run_blocking(&self) -> Result<(), tonic::Status> {
        loop {
            self.connect_and_serve().await?;
            log::warn!("metrics tunnel connection lost, reconnecting");
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
        request.metadata_mut().insert("api_key", api_key);

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
                    let response = build_response(&req_data, &self.storage_engine).await;
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
                response: None,
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
