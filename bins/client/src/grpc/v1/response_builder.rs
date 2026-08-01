//! One builder per metric kind. Each queries the storage engine for the
//! requested slice of history and maps the rows onto its proto response type,
//! so `metrics_tunnel` only has to dispatch on the request kind.

use crate::grpc::v1;
use crate::grpc::v1::metrics::{ContainerRuntimeMetrics, CpuMetrics};
use crate::grpc::v1::metrics_response::ReturnedMetric::CpuResponse;
use crate::grpc::v1::metrics_tunnel::QueryRange;
use crate::grpc::v1::{metrics, metrics_response};
use crate::storage_engine::storage_engine::StorageEngine;
use anyhow::Result;
use std::sync::Arc;

pub async fn build_cpu_response(
    query_time: QueryRange,
    storage_engine: Arc<StorageEngine>,
) -> Result<metrics_response::ReturnedMetric> {
    // maps the data from the db directly to grpc type
    let items: Vec<CpuMetrics> = match query_time {
        QueryRange::Between(start, end) => storage_engine.get_cpu_stats_between(start, end).await?,
        QueryRange::LastN(n) => storage_engine.get_cpu_stats_latest(n).await?,
    }
    .into_iter()
    .map(v1::metrics_mapping::cpu_metrics)
    .collect();

    Ok(CpuResponse(metrics::CpuResponse { items }))
}

pub async fn build_memory_response(
    query_time: QueryRange,
    storage_engine: Arc<StorageEngine>,
) -> Result<metrics_response::ReturnedMetric> {
    let items = match query_time {
        QueryRange::Between(start, end) => {
            storage_engine.get_memory_stats_between(start, end).await?
        }
        QueryRange::LastN(n) => storage_engine.get_memory_stats_latest(n).await?,
    }
    .into_iter()
    .map(v1::metrics_mapping::memory_metrics)
    .collect();

    Ok(metrics_response::ReturnedMetric::MemoryResponse(
        metrics::MemoryResponse { items },
    ))
}

pub async fn build_disk_response(
    query_time: QueryRange,
    storage_engine: Arc<StorageEngine>,
) -> Result<metrics_response::ReturnedMetric> {
    let items = match query_time {
        QueryRange::Between(start, end) => {
            storage_engine.get_disk_stats_between(start, end).await?
        }
        QueryRange::LastN(n) => storage_engine.get_disk_stats_latest(n).await?,
    }
    .into_iter()
    .map(v1::metrics_mapping::disk_entry)
    .collect();

    Ok(metrics_response::ReturnedMetric::DiskResponse(
        metrics::DiskResponse { items },
    ))
}

pub async fn build_network_response(
    query_time: QueryRange,
    storage_engine: Arc<StorageEngine>,
) -> Result<metrics_response::ReturnedMetric> {
    let items = match query_time {
        QueryRange::Between(start, end) => {
            storage_engine.get_network_stats_between(start, end).await?
        }
        QueryRange::LastN(n) => storage_engine.get_network_stats_latest(n).await?,
    }
    .into_iter()
    .map(v1::metrics_mapping::network_metrics)
    .collect();

    Ok(metrics_response::ReturnedMetric::NetworkResponse(
        metrics::NetworkResponse { items },
    ))
}

pub async fn build_system_response(
    query_time: QueryRange,
    storage_engine: Arc<StorageEngine>,
) -> Result<metrics_response::ReturnedMetric> {
    let items = match query_time {
        QueryRange::Between(start, end) => {
            storage_engine.get_system_stats_between(start, end).await?
        }
        QueryRange::LastN(n) => storage_engine.get_system_stats_latest(n).await?,
    }
    .into_iter()
    .map(v1::metrics_mapping::system_metrics)
    .collect();

    Ok(metrics_response::ReturnedMetric::SystemResponse(
        metrics::SystemResponse { items },
    ))
}

pub async fn build_speedtest_response(
    query_time: QueryRange,
    storage_engine: Arc<StorageEngine>,
) -> Result<metrics_response::ReturnedMetric> {
    let items = match query_time {
        QueryRange::Between(start, end) => {
            storage_engine
                .get_speedtest_stats_between(start, end)
                .await?
        }
        QueryRange::LastN(n) => storage_engine.get_speedtest_stats_latest(n).await?,
    }
    .into_iter()
    .map(v1::metrics_mapping::speedtest_metrics)
    .collect();

    Ok(metrics_response::ReturnedMetric::SpeedtestResponse(
        metrics::SpeedtestResponse { items },
    ))
}

/// `number_of_processes` caps each run's top-cpu/top-memory lists. Proto3 cannot
/// tell an unset int32 from an explicit zero, so <= 0 is treated as "no cap".
pub async fn build_process_response(
    query_time: QueryRange,
    number_of_processes: i32,
    storage_engine: Arc<StorageEngine>,
) -> Result<metrics_response::ReturnedMetric> {
    let items = match query_time {
        QueryRange::Between(start, end) => {
            storage_engine
                .get_processes_stats_between(start, end)
                .await?
        }
        QueryRange::LastN(n) => storage_engine.get_processes_stats_latest(n).await?,
    }
    .into_iter()
    .map(v1::metrics_mapping::processes_stats)
    .map(|mut stats| {
        if number_of_processes > 0 {
            stats.top_cpu.truncate(number_of_processes as usize);
            stats.top_memory.truncate(number_of_processes as usize);
        }
        stats
    })
    .collect();

    Ok(metrics_response::ReturnedMetric::ProcessResponse(
        metrics::ProcessResponse { items },
    ))
}

/// `container_id` picks a single container out of every stored run; `None`
/// returns each run whole.
pub async fn build_container_runtime_response(
    query_time: QueryRange,
    container_id: Option<&str>,
    storage_engine: Arc<StorageEngine>,
) -> Result<metrics_response::ReturnedMetric> {
    let metrics = match query_time {
        QueryRange::Between(start, end) => {
            storage_engine
                .get_container_runtime_stats_between(start, end)
                .await?
        }
        QueryRange::LastN(n) => storage_engine.get_container_runtime_stats_latest(n).await?,
    }
    .into_iter()
    .map(v1::metrics_mapping::container_runtime_stats);

    let response = match container_id {
        Some(id) => metrics::container_runtime_response::Response::ContainerMetricsList(
            metrics::ContainerMetricsList {
                container_metrics: metrics
                    .flat_map(|metric| metric.containers)
                    .filter(|container| container.container_id == id)
                    .collect(),
            },
        ),
        None => metrics::container_runtime_response::Response::ContainerRuntimeMetricsList(
            metrics::ContainerRuntimeMetricsList {
                items: metrics.collect(),
            },
        ),
    };

    Ok(metrics_response::ReturnedMetric::ContainerRuntimeResponse(
        metrics::ContainerRuntimeResponse {
            response: Some(response),
        },
    ))
}

/// Runs every per-metric builder concurrently and collects them into a single
/// `FullResponse`. Each builder returns the `returned_metric` variant it owns,
/// so filling the struct is just a matter of putting each one in its slot.
/// A metric whose query fails is logged and left unset rather than failing the
/// whole response, so one bad table cannot blank out the others.
pub async fn build_full_response(
    query_time: QueryRange,
    storage_engine: Arc<StorageEngine>,
) -> Result<metrics_response::ReturnedMetric> {
    let (cpu, memory, disk, network, system, speedtest, process, containers) = tokio::join!(
        build_cpu_response(query_time, storage_engine.clone()),
        build_memory_response(query_time, storage_engine.clone()),
        build_disk_response(query_time, storage_engine.clone()),
        build_network_response(query_time, storage_engine.clone()),
        build_system_response(query_time, storage_engine.clone()),
        build_speedtest_response(query_time, storage_engine.clone()),
        build_process_response(query_time, 0, storage_engine.clone()),
        build_container_runtime_response(query_time, None, storage_engine.clone()),
    );

    let mut full = v1::FullResponse::default();
    for built in [
        cpu, memory, disk, network, system, speedtest, process, containers,
    ] {
        let metric = match built {
            Ok(metric) => metric,
            Err(e) => {
                log::error!("failed to build part of full metrics: {e}");
                continue;
            }
        };

        use metrics_response::ReturnedMetric as R;
        match metric {
            R::CpuResponse(r) => full.cpu_response = Some(r),
            R::MemoryResponse(r) => full.memory_response = Some(r),
            R::DiskResponse(r) => full.disk_response = Some(r),
            R::NetworkResponse(r) => full.network_response = Some(r),
            R::SystemResponse(r) => full.system_response = Some(r),
            R::SpeedtestResponse(r) => full.speedtest_response = Some(r),
            R::ProcessResponse(r) => full.process_response = Some(r),
            R::ContainerRuntimeResponse(r) => full.container_runtime_response = Some(r),
            R::FullResponse(_) => log::warn!("nested full response in full metrics, ignoring"),
        }
    }

    Ok(metrics_response::ReturnedMetric::FullResponse(full))
}
