use crate::grpc::v1::metrics::ContainerRuntimeRequest;
use crate::grpc::v1::metrics::container_runtime_request::MessageType;
use crate::grpc::v1::metrics::container_runtime_response;
use crate::grpc::v1::metrics_mapping;
use crate::grpc::v1::metrics_request::RequestedMetric;
use crate::grpc::v1::metrics_response::ReturnedMetric;
use crate::grpc::v1::query_range::QueryRange;
use crate::grpc::v1::statistics::{
    RecapActivity, RecapAvailability, RecapCpuUsage, RecapNetwork, RecapRequest, RecapResponse,
    RecapStat,
};
use crate::grpc::v1::{FullRequest, MetricsRequest, MetricsResponse, TimeQuery, TimeRange};
use crate::storage_engine::storage_engine::StorageEngine;
use chrono::{DateTime, TimeZone, Utc};
use std::sync::Arc;

pub async fn build_metrics_response(
    storage_engine: Arc<StorageEngine>,
    request: MetricsRequest,
) -> MetricsResponse {
    let request_id = request.request_id.clone();

    let requested_metric = request
        .requested_metric
        .unwrap_or(RequestedMetric::FullRequest(FullRequest {
            time_query: None,
        }));
    let returned_metric = match requested_metric {
        RequestedMetric::CpuRequest(r) => {
            build_cpu_response(storage_engine, range_of(&r.time_query)).await
        }
        RequestedMetric::PartitionRequest(r) => {
            build_partition_response(storage_engine, range_of(&r.time_query)).await
        }
        RequestedMetric::MemoryRequest(r) => {
            build_memory_response(storage_engine, range_of(&r.time_query)).await
        }
        RequestedMetric::NetworkRequest(r) => {
            build_network_response(storage_engine, range_of(&r.time_query)).await
        }
        RequestedMetric::ProcessRequest(r) => {
            build_process_response(storage_engine, range_of(&r.time_query)).await
        }
        RequestedMetric::SpeedtestRequest(r) => {
            build_speedtest_response(storage_engine, range_of(&r.time_query)).await
        }
        RequestedMetric::SystemRequest(r) => {
            build_system_response(storage_engine, range_of(&r.time_query)).await
        }
        RequestedMetric::ContainerRuntimeRequest(r) => {
            let query_range = range_of(&r.time_query);
            build_container_runtime_response(storage_engine, query_range, r).await
        }
        RequestedMetric::ErrorStatsRequest(r) => {
            build_error_stats_response(storage_engine, range_of(&r.time_query)).await
        }
        RequestedMetric::TunnelAccessLogRequest(r) => {
            build_tunnel_access_log_response(storage_engine, range_of(&r.time_query)).await
        }
        RequestedMetric::RecapRequest(r) => build_recap_response(storage_engine, r).await,
        RequestedMetric::FullRequest(r) => {
            build_full_response(storage_engine, range_of(&r.time_query)).await
        }
    };

    MetricsResponse {
        request_id,
        returned_metric,
    }
}

fn range_of(time_query: &Option<TimeQuery>) -> QueryRange {
    QueryRange::from_time_query(time_query.as_ref())
}

/// A gap between two system samples larger than this counts as a disconnect.
const RECAP_GAP_THRESHOLD_SECS: i64 = 180;

fn recap_range(range: &Option<TimeRange>) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let range = range.as_ref()?;
    Some((
        Utc.timestamp_opt(range.start, 0).single()?,
        Utc.timestamp_opt(range.end, 0).single()?,
    ))
}

/// All recap values of one range: (uptime, disconnects, avg cpu, spikes, idle/load, bytes in,
/// bytes out, downlink B/s, uplink B/s, tunnel accesses, errors).
type RecapValues = (
    Option<f64>,
    Option<u64>,
    Option<f64>,
    Option<u64>,
    Option<f64>,
    Option<i64>,
    Option<i64>,
    Option<f64>,
    Option<f64>,
    u64,
    u64,
);

async fn recap_values(
    storage_engine: &StorageEngine,
    (start, end): (DateTime<Utc>, DateTime<Utc>),
) -> anyhow::Result<RecapValues> {
    let gap = RECAP_GAP_THRESHOLD_SECS;
    tokio::try_join!(
        storage_engine.uptime_percent_over_time(start, end, gap),
        storage_engine.disconnect_count_over_time(start, end, gap),
        storage_engine.cpu_avg_percent_over_time(start, end),
        storage_engine.cpu_load_spike_count_over_time(start, end),
        storage_engine.cpu_idle_to_load_ratio_over_time(start, end),
        storage_engine.network_total_bytes_in_over_time(start, end),
        storage_engine.network_total_bytes_out_over_time(start, end),
        storage_engine.network_avg_downlink_bytes_per_sec_over_time(start, end),
        storage_engine.network_avg_uplink_bytes_per_sec_over_time(start, end),
        storage_engine.tunnel_access_count_over_time(start, end),
        storage_engine.error_count_over_time(start, end),
    )
}

async fn build_recap_response(
    storage_engine: Arc<StorageEngine>,
    request: RecapRequest,
) -> Option<ReturnedMetric> {
    let query = request.recap_query?;
    let (Some(one), Some(two)) = (recap_range(&query.range_one), recap_range(&query.range_two))
    else {
        log::warn!("recap request without two valid time ranges");
        return None;
    };

    let (one, two) = match tokio::try_join!(
        recap_values(&storage_engine, one),
        recap_values(&storage_engine, two)
    ) {
        Ok(values) => values,
        Err(e) => {
            log::error!("failed to build recap response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::RecapResponse(RecapResponse {
        availability: Some(RecapAvailability {
            uptime_percent: Some(RecapStat {
                range_one: one.0,
                range_two: two.0,
            }),
            disconnects: Some(RecapStat {
                range_one: one.1.map(|v| v as f64),
                range_two: two.1.map(|v| v as f64),
            }),
        }),
        cpu_usage: Some(RecapCpuUsage {
            avg_usage_percent: Some(RecapStat {
                range_one: one.2,
                range_two: two.2,
            }),
            load_spikes: Some(RecapStat {
                range_one: one.3.map(|v| v as f64),
                range_two: two.3.map(|v| v as f64),
            }),
            idle_to_load_ratio: Some(RecapStat {
                range_one: one.4,
                range_two: two.4,
            }),
        }),
        network: Some(RecapNetwork {
            total_in_gb: Some(RecapStat {
                range_one: one.5.map(|v| v as f64 / 1e9),
                range_two: two.5.map(|v| v as f64 / 1e9),
            }),
            total_out_gb: Some(RecapStat {
                range_one: one.6.map(|v| v as f64 / 1e9),
                range_two: two.6.map(|v| v as f64 / 1e9),
            }),
            avg_downlink_mb_s: Some(RecapStat {
                range_one: one.7.map(|v| v / 1e6),
                range_two: two.7.map(|v| v / 1e6),
            }),
            avg_uplink_mb_s: Some(RecapStat {
                range_one: one.8.map(|v| v / 1e6),
                range_two: two.8.map(|v| v / 1e6),
            }),
        }),
        activity: Some(RecapActivity {
            tunnel_access_count: Some(RecapStat {
                range_one: Some(one.9 as f64),
                range_two: Some(two.9 as f64),
            }),
            error_count: Some(RecapStat {
                range_one: Some(one.10 as f64),
                range_two: Some(two.10 as f64),
            }),
        }),
    }))
}

async fn build_full_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let full_container_request = ContainerRuntimeRequest {
        message_type: Some(MessageType::FullMetrics(
            crate::grpc::v1::metrics::GetFullMetrics {},
        )),
        time_query: None,
    };

    let (cpu, partition, memory, network, system, speedtest, process, containers, errors, tunnel) = tokio::join!(
        build_cpu_response(storage_engine.clone(), query_range),
        build_partition_response(storage_engine.clone(), query_range),
        build_memory_response(storage_engine.clone(), query_range),
        build_network_response(storage_engine.clone(), query_range),
        build_system_response(storage_engine.clone(), query_range),
        build_speedtest_response(storage_engine.clone(), query_range),
        build_process_response(storage_engine.clone(), query_range),
        build_container_runtime_response(
            storage_engine.clone(),
            query_range,
            full_container_request
        ),
        build_error_stats_response(storage_engine.clone(), query_range),
        build_tunnel_access_log_response(storage_engine.clone(), query_range),
    );

    let mut full = crate::grpc::v1::FullResponse::default();
    for built in [
        cpu, partition, memory, network, system, speedtest, process, containers, errors, tunnel,
    ] {
        match built {
            Some(ReturnedMetric::CpuResponse(r)) => full.cpu_response = Some(r),
            Some(ReturnedMetric::PartitionResponse(r)) => full.partition_response = Some(r),
            Some(ReturnedMetric::MemoryResponse(r)) => full.memory_response = Some(r),
            Some(ReturnedMetric::NetworkResponse(r)) => full.network_response = Some(r),
            Some(ReturnedMetric::SystemResponse(r)) => full.system_response = Some(r),
            Some(ReturnedMetric::SpeedtestResponse(r)) => full.speedtest_response = Some(r),
            Some(ReturnedMetric::ProcessResponse(r)) => full.process_response = Some(r),
            Some(ReturnedMetric::ContainerRuntimeResponse(r)) => {
                full.container_runtime_response = Some(r)
            }
            Some(ReturnedMetric::ErrorStatsResponse(r)) => full.error_stats_response = Some(r),
            Some(ReturnedMetric::TunnelAccessLogResponse(r)) => {
                full.tunnel_access_log_response = Some(r)
            }
            Some(ReturnedMetric::RecapResponse(_)) => {
                log::warn!("recap response in full metrics, ignoring")
            }
            Some(ReturnedMetric::FullResponse(_)) => {
                log::warn!("nested full response in full metrics, ignoring")
            }
            None => {}
        }
    }

    Some(ReturnedMetric::FullResponse(full))
}

async fn build_tunnel_access_log_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => {
            storage_engine
                .get_tunnel_access_log_between(start, end)
                .await
        }
        QueryRange::LastN(n) => storage_engine.get_tunnel_access_log_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(metrics_mapping::tunnel_access_log_metrics)
            .collect(),
        Err(e) => {
            log::error!("failed to build tunnel access log response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::TunnelAccessLogResponse(
        crate::grpc::v1::metrics::TunnelAccessLogResponse { items },
    ))
}

async fn build_container_runtime_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
    request: ContainerRuntimeRequest,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => {
            storage_engine
                .get_container_runtime_stats_between(start, end)
                .await
        }
        QueryRange::LastN(n) => storage_engine.get_container_runtime_stats_latest(n).await,
    };

    let metrics = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(metrics_mapping::container_runtime_stats),
        Err(e) => {
            log::error!("failed to build container runtime response: {e}");
            return None;
        }
    };

    let container_id = match request.message_type {
        Some(MessageType::OneContainerMetrics(one)) => Some(one.container_id),
        _ => None,
    };

    let response = match container_id {
        Some(id) => container_runtime_response::Response::ContainerMetricsList(
            crate::grpc::v1::metrics::ContainerMetricsList {
                container_metrics: metrics
                    .flat_map(|metric| metric.containers)
                    .filter(|container| container.container_id == id)
                    .collect(),
            },
        ),
        None => container_runtime_response::Response::ContainerRuntimeMetricsList(
            crate::grpc::v1::metrics::ContainerRuntimeMetricsList {
                items: metrics.collect(),
            },
        ),
    };

    Some(ReturnedMetric::ContainerRuntimeResponse(
        crate::grpc::v1::metrics::ContainerRuntimeResponse {
            response: Some(response),
        },
    ))
}

async fn build_error_stats_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_error_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_error_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(metrics_mapping::error_stats_metrics)
            .collect(),
        Err(e) => {
            log::error!("failed to build error stats response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::ErrorStatsResponse(
        crate::grpc::v1::metrics::ErrorStatsResponse { items },
    ))
}

async fn build_system_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => {
            storage_engine.get_system_stats_between(start, end).await
        }
        QueryRange::LastN(n) => storage_engine.get_system_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(metrics_mapping::system_metrics)
            .collect(),
        Err(e) => {
            log::error!("failed to build system response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::SystemResponse(
        crate::grpc::v1::metrics::SystemResponse { items },
    ))
}

async fn build_speedtest_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => {
            storage_engine.get_speedtest_stats_between(start, end).await
        }
        QueryRange::LastN(n) => storage_engine.get_speedtest_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(metrics_mapping::speedtest_metrics)
            .collect(),
        Err(e) => {
            log::error!("failed to build speedtest response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::SpeedtestResponse(
        crate::grpc::v1::metrics::SpeedtestResponse { items },
    ))
}

async fn build_process_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => {
            storage_engine.get_processes_stats_between(start, end).await
        }
        QueryRange::LastN(n) => storage_engine.get_processes_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(metrics_mapping::processes_stats)
            .collect(),
        Err(e) => {
            log::error!("failed to build process response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::ProcessResponse(
        crate::grpc::v1::metrics::ProcessResponse { items },
    ))
}

async fn build_network_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => {
            storage_engine.get_network_stats_between(start, end).await
        }
        QueryRange::LastN(n) => storage_engine.get_network_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(metrics_mapping::network_metrics)
            .collect(),
        Err(e) => {
            log::error!("failed to build network response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::NetworkResponse(
        crate::grpc::v1::metrics::NetworkResponse { items },
    ))
}

async fn build_memory_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => {
            storage_engine.get_memory_stats_between(start, end).await
        }
        QueryRange::LastN(n) => storage_engine.get_memory_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(metrics_mapping::memory_metrics)
            .collect(),
        Err(e) => {
            log::error!("failed to build memory response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::MemoryResponse(
        crate::grpc::v1::metrics::MemoryResponse { items },
    ))
}

async fn build_partition_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => {
            storage_engine.get_partition_stats_between(start, end).await
        }
        QueryRange::LastN(n) => storage_engine.get_partition_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(metrics_mapping::partition_entry)
            .collect(),
        Err(e) => {
            log::error!("failed to build partition response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::PartitionResponse(
        crate::grpc::v1::metrics::PartitionResponse { items },
    ))
}

async fn build_cpu_response(
    storage_engine: Arc<StorageEngine>,
    query_range: QueryRange,
) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_cpu_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_cpu_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::cpu_metrics).collect(),
        Err(e) => {
            log::error!("failed to build cpu response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::CpuResponse(
        crate::grpc::v1::metrics::CpuResponse { items },
    ))
}
