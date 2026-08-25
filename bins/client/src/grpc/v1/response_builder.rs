use std::sync::Arc;
use crate::grpc::v1::{FullRequest, MetricsRequest, MetricsResponse};
use crate::grpc::v1::metrics::ContainerRuntimeRequest;
use crate::grpc::v1::metrics_request::RequestedMetric;
use crate::grpc::v1::query_range::QueryRange;
use crate::storage_engine::storage_engine::StorageEngine;

pub async fn build_metrics_response(storage_engine: Arc<StorageEngine>, request: MetricsRequest) -> MetricsResponse {
    let query_range = QueryRange::from_request(&request);

    let requested_metric = request.requested_metric.unwrap_or(RequestedMetric::FullRequest(FullRequest{}));
    match requested_metric {
        RequestedMetric::CpuRequest(_) => {build_cpu_response(storage_engine, query_range)}
        RequestedMetric::PartitionRequest(_) => {build_partition_response(storage_engine, query_range)}
        RequestedMetric::MemoryRequest(_) => {build_memory_response(storage_engine, query_range)}
        RequestedMetric::NetworkRequest(_) => {build_network_response(storage_engine, query_range)}
        RequestedMetric::ProcessRequest(_) => {build_process_response(storage_engine, query_range)}
        RequestedMetric::SpeedtestRequest(_) => {build_speedtest_response(storage_engine, query_range)}
        RequestedMetric::SystemRequest(_) => {build_system_response(storage_engine, query_range)}
        RequestedMetric::ContainerRuntimeRequest(req) => {build_container_runtime_response(storage_engine, query_range, req)}
        RequestedMetric::ErrorStatsRequest(_) => {build_error_stats_response(storage_engine, query_range)}
        RequestedMetric::TunnelAccessLogRequest(_) => {build_tunnel_access_log_response(storage_engine, query_range)}
        RequestedMetric::FullRequest(_) => {build_full_response(storage_engine, query_range)}
    }


    todo!()
}

fn build_full_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}

fn build_tunnel_access_log_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}

fn build_container_runtime_response(p0: Arc<StorageEngine>, p1: QueryRange, p2: ContainerRuntimeRequest) {
    todo!()
}

fn build_error_stats_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}

fn build_system_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}

fn build_speedtest_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}

fn build_process_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}

fn build_network_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}

fn build_memory_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}

fn build_partition_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}

fn build_cpu_response(p0: Arc<StorageEngine>, p1: QueryRange) {
    todo!()
}
/*
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

    Ok(CpuResponse(CpuResponse { items }))
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
        MemoryResponse { items },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{minutes_ago, TestDb};
    use metrics_response::ReturnedMetric as R;

    fn all_of_it() -> QueryRange {
        QueryRange::Between(minutes_ago(60), minutes_ago(0))
    }

    #[tokio::test]
    async fn cpu_response_returns_both_seeded_cycles() {
        let db = TestDb::new().await;
        db.seed_two_cycles().await;

        let R::CpuResponse(cpu) = build_cpu_response(all_of_it(), db.engine()).await.unwrap()
        else {
            panic!("expected a cpu response");
        };

        assert_eq!(cpu.items.len(), 2, "both seeded cycles should come back");
        assert_eq!(cpu.items[0].name, "test-cpu");
        assert_eq!(cpu.items[0].count, 8);
        assert_eq!(cpu.items[0].cores.len(), 2, "cores are joined onto the cpu row");
    }

    #[tokio::test]
    async fn last_n_returns_only_the_newest_cycle() {
        let db = TestDb::new().await;
        let (_older, newer) = db.seed_two_cycles().await;

        let R::CpuResponse(cpu) = build_cpu_response(QueryRange::LastN(1), db.engine())
            .await
            .unwrap()
        else {
            panic!("expected a cpu response");
        };

        assert_eq!(cpu.items.len(), 1);
        let collected_at = cpu.items[0].collected_at.as_ref().unwrap();
        assert_eq!(
            collected_at.seconds,
            newer.timestamp(),
            "last_n should return the newer cycle, not the older one"
        );
    }

    #[tokio::test]
    async fn range_excluding_the_older_cycle_returns_one() {
        let db = TestDb::new().await;
        let (older, newer) = db.seed_two_cycles().await;

        // anchored to the seeded timestamps rather than a fresh `now`, so the
        // boundary lands between the two cycles no matter when the test runs
        let range = QueryRange::Between(
            older + chrono::Duration::seconds(1),
            newer + chrono::Duration::seconds(1),
        );
        let R::MemoryResponse(memory) = build_memory_response(range, db.engine()).await.unwrap()
        else {
            panic!("expected a memory response");
        };

        assert_eq!(memory.items.len(), 1);
        assert_eq!(
            memory.items[0].collected_at.as_ref().unwrap().seconds,
            newer.timestamp()
        );
    }

    #[tokio::test]
    async fn empty_database_yields_empty_items_not_an_error() {
        let db = TestDb::new().await;

        let R::CpuResponse(cpu) = build_cpu_response(all_of_it(), db.engine()).await.unwrap()
        else {
            panic!("expected a cpu response");
        };

        assert!(cpu.items.is_empty());
    }

    #[tokio::test]
    async fn disk_network_system_and_speedtest_come_back_populated() {
        let db = TestDb::new().await;
        db.seed_two_cycles().await;
        let engine = db.engine();

        let R::DiskResponse(disk) = build_disk_response(all_of_it(), engine.clone()).await.unwrap()
        else {
            panic!("expected a disk response");
        };
        assert_eq!(disk.items.len(), 2);
        assert_eq!(disk.items[0].disks.len(), 1, "one disk per seeded cycle");
        assert_eq!(disk.items[0].disks[0].name, "/dev/test0");

        let R::NetworkResponse(network) =
            build_network_response(all_of_it(), engine.clone()).await.unwrap()
        else {
            panic!("expected a network response");
        };
        assert_eq!(network.items.len(), 2);
        assert_eq!(network.items[0].local_ip, "192.168.1.10");

        let R::SystemResponse(system) =
            build_system_response(all_of_it(), engine.clone()).await.unwrap()
        else {
            panic!("expected a system response");
        };
        assert_eq!(system.items.len(), 2);
        assert_eq!(system.items[0].host_name.as_deref(), Some("test-host"));

        let R::SpeedtestResponse(speedtest) =
            build_speedtest_response(all_of_it(), engine).await.unwrap()
        else {
            panic!("expected a speedtest response");
        };
        assert_eq!(speedtest.items.len(), 2);
        assert_eq!(speedtest.items[0].download_mbps, 100.5);
    }

    #[tokio::test]
    async fn process_response_caps_each_cycle_at_the_requested_count() {
        let db = TestDb::new().await;
        db.seed_two_cycles().await; // 3 cpu + 3 memory entries per cycle

        let R::ProcessResponse(capped) = build_process_response(all_of_it(), 2, db.engine())
            .await
            .unwrap()
        else {
            panic!("expected a process response");
        };

        assert_eq!(capped.items.len(), 2, "one entry per seeded cycle");
        for stats in &capped.items {
            assert_eq!(stats.top_cpu.len(), 2, "capped from 3 down to 2");
            assert_eq!(stats.top_memory.len(), 2);
        }
    }

    #[tokio::test]
    async fn process_count_of_zero_means_no_cap() {
        let db = TestDb::new().await;
        db.seed_two_cycles().await;

        let R::ProcessResponse(uncapped) = build_process_response(all_of_it(), 0, db.engine())
            .await
            .unwrap()
        else {
            panic!("expected a process response");
        };

        for stats in &uncapped.items {
            assert_eq!(stats.top_cpu.len(), 3, "zero is treated as unset, so no cap");
            assert_eq!(stats.top_memory.len(), 3);
        }
    }

    #[tokio::test]
    async fn container_request_without_an_id_returns_whole_cycles() {
        let db = TestDb::new().await;
        db.seed_two_cycles().await; // two containers per cycle

        let R::ContainerRuntimeResponse(response) =
            build_container_runtime_response(all_of_it(), None, db.engine())
                .await
                .unwrap()
        else {
            panic!("expected a container runtime response");
        };

        let Some(metrics::container_runtime_response::Response::ContainerRuntimeMetricsList(list)) =
            response.response
        else {
            panic!("expected the whole-cycle variant");
        };

        assert_eq!(list.items.len(), 2, "one entry per seeded cycle");
        assert_eq!(list.items[0].containers.len(), 2, "both containers in the cycle");
    }

    #[tokio::test]
    async fn container_request_with_an_id_flattens_to_that_containers_history() {
        let db = TestDb::new().await;
        db.seed_two_cycles().await;

        let R::ContainerRuntimeResponse(response) =
            build_container_runtime_response(all_of_it(), Some("container-a"), db.engine())
                .await
                .unwrap()
        else {
            panic!("expected a container runtime response");
        };

        let Some(metrics::container_runtime_response::Response::ContainerMetricsList(list)) =
            response.response
        else {
            panic!("expected the single-container variant");
        };

        // one entry per cycle, and container-b is filtered out
        assert_eq!(list.container_metrics.len(), 2);
        assert!(
            list.container_metrics
                .iter()
                .all(|c| c.container_id == "container-a"),
            "only the requested container should survive the filter"
        );
    }

    #[tokio::test]
    async fn full_response_fills_every_slot() {
        let db = TestDb::new().await;
        db.seed_two_cycles().await;

        let R::FullResponse(full) = build_full_response(all_of_it(), db.engine()).await.unwrap()
        else {
            panic!("expected a full response");
        };

        assert_eq!(full.cpu_response.unwrap().items.len(), 2);
        assert_eq!(full.memory_response.unwrap().items.len(), 2);
        assert_eq!(full.disk_response.unwrap().items.len(), 2);
        assert_eq!(full.network_response.unwrap().items.len(), 2);
        assert_eq!(full.system_response.unwrap().items.len(), 2);
        assert_eq!(full.speedtest_response.unwrap().items.len(), 2);
        assert_eq!(full.process_response.unwrap().items.len(), 2);
        assert!(
            full.container_runtime_response.is_some(),
            "containers should be filled in too"
        );
    }
}
*/
