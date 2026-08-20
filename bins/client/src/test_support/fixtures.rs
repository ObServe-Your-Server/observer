use super::TestDb;
use crate::entities::{process_stats, processes_stats};
use crate::jobs::base_metric_collection_job::BaseMetrics;
use chrono::{DateTime, Utc};
use open_eye::collector::container_runtime::collector::{
    ContainerRuntime, ContainerRuntimeStats, ContainerStats,
};
use open_eye::collector::cpu::collector::{Core, CpuStats};
use open_eye::collector::disk::collector::DiskInfo;
use open_eye::collector::memory::collector::MemoryStats;
use open_eye::collector::network::collector::NetworkStats;
use open_eye::collector::speedtest::collector::SpeedtestResult;
use open_eye::collector::systemstats::collector::SystemStats;
use sea_orm::{ActiveValue::Set, EntityTrait};

pub fn sample_cpu(collected_at: DateTime<Utc>) -> CpuStats {
    CpuStats {
        cpu_name: "test-cpu".to_string(),
        cpu_count: 8,
        cpu_physical_count: 4,
        cpu_usage_percent: 25.5,
        cpu_temperature_celsius: 45.0,
        core_information: (0..2)
            .map(|i| Core {
                core_name: format!("core-{i}"),
                core_usage_percent: 10.0 + i as f32,
                core_frequency_mhz: 3200,
            })
            .collect(),
        collected_at,
    }
}

pub fn sample_memory(collected_at: DateTime<Utc>) -> MemoryStats {
    MemoryStats {
        total_memory_in_byte: 16_000_000_000,
        available_memory_in_byte: 8_000_000_000,
        used_memory_in_byte: 8_000_000_000,
        total_swap_in_byte: 2_000_000_000,
        available_swap_in_byte: 1_500_000_000,
        used_swap_in_byte: 500_000_000,
        collected_at,
    }
}

pub fn sample_disks(collected_at: DateTime<Utc>) -> Vec<DiskInfo> {
    vec![DiskInfo {
        name: "/dev/test0".to_string(),
        total_bytes: 500_000_000_000,
        used_bytes: 200_000_000_000,
        available_bytes: 300_000_000_000,
        used_blocks: 200_000,
        available_blocks: 300_000,
        block_size: 4096,
        collected_at,
    }]
}

pub fn sample_network(collected_at: DateTime<Utc>) -> NetworkStats {
    NetworkStats {
        local_ip: "192.168.1.10".to_string(),
        total_bytes_transmitted: 1_000_000,
        total_bytes_received: 2_000_000,
        total_packets_transmitted: 1_000,
        total_packets_received: 2_000,
        collected_at,
    }
}

pub fn sample_system(collected_at: DateTime<Utc>) -> SystemStats {
    SystemStats {
        os_name: Some("test-os".to_string()),
        uptime_seconds: 3600,
        host_name: Some("test-host".to_string()),
        kernel_version: "1.2.3-test".to_string(),
        collected_at,
    }
}

pub fn sample_speedtest(collected_at: DateTime<Utc>) -> SpeedtestResult {
    SpeedtestResult {
        download_mbps: 100.5,
        upload_mbps: 50.25,
        ping_ms: 12.5,
        collected_at,
    }
}

pub fn sample_container_runtime(
    collected_at: DateTime<Utc>,
    container_ids: &[&str],
) -> ContainerRuntimeStats {
    ContainerRuntimeStats {
        collected_at,
        container_stats: container_ids
            .iter()
            .enumerate()
            .map(|(i, id)| ContainerStats {
                container_runtime: ContainerRuntime::Docker,
                id: (*id).to_string(),
                host_name: format!("host-{id}"),
                created_at: collected_at.timestamp(),
                status: "running".to_string(),
                running: true,
                running_for_seconds: 600,
                image_name: format!("image-{id}:latest"),
                networks: vec!["bridge".to_string()],
                cpu_usage_percent: 5.0 + i as f64,
                memory_usage_bytes: 100_000_000 + i as u64 * 1_000_000,
                collected_at,
            })
            .collect(),
    }
}
pub fn sample_base_metrics(collected_at: DateTime<Utc>) -> BaseMetrics {
    BaseMetrics {
        cpu: Some(sample_cpu(collected_at)),
        memory: Some(sample_memory(collected_at)),
        disks: Some(sample_disks(collected_at)),
        network: Some(sample_network(collected_at)),
        system: Some(sample_system(collected_at)),
    }
}

impl TestDb {
    pub async fn seed_base_metrics(&self, collected_at: DateTime<Utc>) {
        self.engine()
            .save_base_metrics_to_db(sample_base_metrics(collected_at))
            .await
            .expect("failed to seed base metrics");
    }

    pub async fn seed_speedtest(&self, collected_at: DateTime<Utc>) {
        self.engine()
            .save_speedtest_stats_to_db(sample_speedtest(collected_at))
            .await
            .expect("failed to seed speedtest stats");
    }

    pub async fn seed_container_runtime(
        &self,
        collected_at: DateTime<Utc>,
        container_ids: &[&str],
    ) {
        self.engine()
            .save_container_runtime_stats_to_db(sample_container_runtime(
                collected_at,
                container_ids,
            ))
            .await
            .expect("failed to seed container runtime stats");
    }

    pub async fn seed_processes(&self, collected_at: DateTime<Utc>, per_kind: usize) {
        let db = self.engine();
        let db = db.db_for_tests().expect("database not initialized");

        let parent = processes_stats::Entity::insert(processes_stats::ActiveModel {
            collected_at: Set(collected_at.into()),
            ..Default::default()
        })
        .exec(db)
        .await
        .expect("failed to seed processes_stats");

        for kind in ["cpu", "memory"] {
            for i in 0..per_kind {
                process_stats::Entity::insert(process_stats::ActiveModel {
                    processes_stats_id: Set(parent.last_insert_id),
                    kind: Set(kind.to_string()),
                    pid: Set(1000 + i as i64),
                    name: Set(format!("{kind}-process-{i}")),
                    user_name: Set("tester".to_string()),
                    status: Set("running".to_string()),
                    // descending so a truncate keeps the heaviest entries first
                    cpu_usage_percent: Set((per_kind - i) as f32),
                    memory_usage_bytes: Set((per_kind - i) as i64 * 1_000_000),
                    ..Default::default()
                })
                .exec(db)
                .await
                .expect("failed to seed process_stats");
            }
        }
    }

    pub async fn seed_all(&self, collected_at: DateTime<Utc>) {
        self.seed_base_metrics(collected_at).await;
        self.seed_speedtest(collected_at).await;
        self.seed_container_runtime(collected_at, &["container-a", "container-b"])
            .await;
        self.seed_processes(collected_at, 3).await;
    }

    pub async fn seed_two_cycles(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        // both derived from a single `now`, so the gap between them is exact and
        // a test can place a range boundary between the two reliably
        let now = Utc::now();
        let older = now - chrono::Duration::minutes(2);
        let newer = now - chrono::Duration::minutes(1);

        self.seed_all(older).await;
        self.seed_all(newer).await;

        (older, newer)
    }
}
