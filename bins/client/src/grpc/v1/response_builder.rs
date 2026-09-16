use std::sync::Arc;
use crate::grpc::v1::{FullRequest, MetricsRequest, MetricsResponse};
use crate::grpc::v1::metrics::ContainerRuntimeRequest;
use crate::grpc::v1::metrics::container_runtime_request::MessageType;
use crate::grpc::v1::metrics::container_runtime_response;
use crate::grpc::v1::metrics_mapping;
use crate::grpc::v1::metrics_request::RequestedMetric;
use crate::grpc::v1::metrics_response::ReturnedMetric;
use crate::grpc::v1::query_range::QueryRange;
use crate::storage_engine::storage_engine::StorageEngine;

pub async fn build_metrics_response(storage_engine: Arc<StorageEngine>, request: MetricsRequest) -> MetricsResponse {
    let query_range = QueryRange::from_request(&request);
    let request_id = request.request_id.clone();

    let requested_metric = request.requested_metric.unwrap_or(RequestedMetric::FullRequest(FullRequest{}));
    let returned_metric = match requested_metric {
        RequestedMetric::CpuRequest(_) => build_cpu_response(storage_engine, query_range).await,
        RequestedMetric::PartitionRequest(_) => build_partition_response(storage_engine, query_range).await,
        RequestedMetric::MemoryRequest(_) => build_memory_response(storage_engine, query_range).await,
        RequestedMetric::NetworkRequest(_) => build_network_response(storage_engine, query_range).await,
        RequestedMetric::ProcessRequest(_) => build_process_response(storage_engine, query_range).await,
        RequestedMetric::SpeedtestRequest(_) => build_speedtest_response(storage_engine, query_range).await,
        RequestedMetric::SystemRequest(_) => build_system_response(storage_engine, query_range).await,
        RequestedMetric::ContainerRuntimeRequest(req) => build_container_runtime_response(storage_engine, query_range, req).await,
        RequestedMetric::ErrorStatsRequest(_) => build_error_stats_response(storage_engine, query_range).await,
        RequestedMetric::TunnelAccessLogRequest(_) => build_tunnel_access_log_response(storage_engine, query_range).await,
        RequestedMetric::FullRequest(_) => build_full_response(storage_engine, query_range).await,
    };

    MetricsResponse {
        request_id,
        returned_metric,
    }
}

async fn build_full_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
    let full_container_request = ContainerRuntimeRequest {
        message_type: Some(MessageType::FullMetrics(
            crate::grpc::v1::metrics::GetFullMetrics {},
        )),
    };

    let (cpu, partition, memory, network, system, speedtest, process, containers, errors, tunnel) = tokio::join!(
        build_cpu_response(storage_engine.clone(), query_range),
        build_partition_response(storage_engine.clone(), query_range),
        build_memory_response(storage_engine.clone(), query_range),
        build_network_response(storage_engine.clone(), query_range),
        build_system_response(storage_engine.clone(), query_range),
        build_speedtest_response(storage_engine.clone(), query_range),
        build_process_response(storage_engine.clone(), query_range),
        build_container_runtime_response(storage_engine.clone(), query_range, full_container_request),
        build_error_stats_response(storage_engine.clone(), query_range),
        build_tunnel_access_log_response(storage_engine.clone(), query_range),
    );

    let mut full = crate::grpc::v1::FullResponse::default();
    for built in [cpu, partition, memory, network, system, speedtest, process, containers, errors, tunnel] {
        match built {
            Some(ReturnedMetric::CpuResponse(r)) => full.cpu_response = Some(r),
            Some(ReturnedMetric::PartitionResponse(r)) => full.partition_response = Some(r),
            Some(ReturnedMetric::MemoryResponse(r)) => full.memory_response = Some(r),
            Some(ReturnedMetric::NetworkResponse(r)) => full.network_response = Some(r),
            Some(ReturnedMetric::SystemResponse(r)) => full.system_response = Some(r),
            Some(ReturnedMetric::SpeedtestResponse(r)) => full.speedtest_response = Some(r),
            Some(ReturnedMetric::ProcessResponse(r)) => full.process_response = Some(r),
            Some(ReturnedMetric::ContainerRuntimeResponse(r)) => full.container_runtime_response = Some(r),
            Some(ReturnedMetric::ErrorStatsResponse(r)) => full.error_stats_response = Some(r),
            Some(ReturnedMetric::TunnelAccessLogResponse(r)) => full.tunnel_access_log_response = Some(r),
            Some(ReturnedMetric::FullResponse(_)) => log::warn!("nested full response in full metrics, ignoring"),
            None => {}
        }
    }

    Some(ReturnedMetric::FullResponse(full))
}

async fn build_tunnel_access_log_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_tunnel_access_log_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_tunnel_access_log_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::tunnel_access_log_metrics).collect(),
        Err(e) => {
            log::error!("failed to build tunnel access log response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::TunnelAccessLogResponse(
        crate::grpc::v1::metrics::TunnelAccessLogResponse { items },
    ))
}

async fn build_container_runtime_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange, request: ContainerRuntimeRequest) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_container_runtime_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_container_runtime_stats_latest(n).await,
    };

    let metrics = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::container_runtime_stats),
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

async fn build_error_stats_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_error_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_error_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::error_stats_metrics).collect(),
        Err(e) => {
            log::error!("failed to build error stats response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::ErrorStatsResponse(
        crate::grpc::v1::metrics::ErrorStatsResponse { items },
    ))
}

async fn build_system_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_system_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_system_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::system_metrics).collect(),
        Err(e) => {
            log::error!("failed to build system response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::SystemResponse(
        crate::grpc::v1::metrics::SystemResponse { items },
    ))
}

async fn build_speedtest_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_speedtest_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_speedtest_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::speedtest_metrics).collect(),
        Err(e) => {
            log::error!("failed to build speedtest response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::SpeedtestResponse(
        crate::grpc::v1::metrics::SpeedtestResponse { items },
    ))
}

async fn build_process_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_processes_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_processes_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::processes_stats).collect(),
        Err(e) => {
            log::error!("failed to build process response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::ProcessResponse(
        crate::grpc::v1::metrics::ProcessResponse { items },
    ))
}

async fn build_network_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_network_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_network_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::network_metrics).collect(),
        Err(e) => {
            log::error!("failed to build network response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::NetworkResponse(
        crate::grpc::v1::metrics::NetworkResponse { items },
    ))
}

async fn build_memory_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_memory_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_memory_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::memory_metrics).collect(),
        Err(e) => {
            log::error!("failed to build memory response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::MemoryResponse(
        crate::grpc::v1::metrics::MemoryResponse { items },
    ))
}

async fn build_partition_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
    let rows = match query_range {
        QueryRange::Between(start, end) => storage_engine.get_partition_stats_between(start, end).await,
        QueryRange::LastN(n) => storage_engine.get_partition_stats_latest(n).await,
    };

    let items = match rows {
        Ok(rows) => rows.into_iter().map(metrics_mapping::partition_entry).collect(),
        Err(e) => {
            log::error!("failed to build partition response: {e}");
            return None;
        }
    };

    Some(ReturnedMetric::PartitionResponse(
        crate::grpc::v1::metrics::PartitionResponse { items },
    ))
}

async fn build_cpu_response(storage_engine: Arc<StorageEngine>, query_range: QueryRange) -> Option<ReturnedMetric> {
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
