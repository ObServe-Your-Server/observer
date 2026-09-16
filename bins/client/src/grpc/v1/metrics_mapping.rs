//! Converts SeaORM entity models (as stored in the DB) into the proto metric types.

use crate::entities::{
    container_runtime_stats, container_stats, cpu_core_stats, cpu_stats, error, memory_stats,
    network_stats, partition_entry, partition_stats, process_stats, processes_stats,
    speedtest_stats, system_stats, tunnel_access_log,
};
use crate::grpc::v1::metrics::{
    ContainerMetrics, ContainerRuntimeMetrics, CoreMetrics, CpuMetrics, ErrorStatsMetrics,
    MemoryMetrics, NetworkMetrics, PartitionEntry, PartitionMetrics, ProcessStats,
    ProcessStatsKind, ProcessesStats, SpeedtestMetrics, SystemMetrics, TunnelAccessLogMetrics,
};

fn to_timestamp(time: chrono::DateTime<chrono::FixedOffset>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: time.timestamp(),
        nanos: time.timestamp_subsec_nanos() as i32,
    }
}

pub fn cpu_metrics(row: (cpu_stats::Model, Vec<cpu_core_stats::Model>)) -> CpuMetrics {
    let (cpu, cores) = row;
    CpuMetrics {
        name: cpu.cpu_name,
        count: cpu.cpu_count as u32,
        physical_count: cpu.cpu_physical_count as u32,
        temperature_celsius: cpu.cpu_temperature_celsius,
        usage_percent: cpu.cpu_usage_percent,
        cores: cores
            .into_iter()
            .map(|core| CoreMetrics {
                name: core.core_name,
                usage_percent: core.core_usage_percent,
                frequency_mhz: core.core_frequency_mhz as f32,
                collected_at: Some(to_timestamp(cpu.collected_at)),
            })
            .collect(),
        collected_at: Some(to_timestamp(cpu.collected_at)),
    }
}

pub fn memory_metrics(memory: memory_stats::Model) -> MemoryMetrics {
    MemoryMetrics {
        total_bytes: memory.total_memory_in_byte as u64,
        available_bytes: memory.available_memory_in_byte as u64,
        used_bytes: memory.used_memory_in_byte as u64,
        total_swap_bytes: memory.total_swap_in_byte as u64,
        available_swap_bytes: memory.available_swap_in_byte as u64,
        used_swap_bytes: memory.used_swap_in_byte as u64,
        collected_at: Some(to_timestamp(memory.collected_at)),
    }
}

pub fn partition_metrics(partition: partition_stats::Model) -> PartitionMetrics {
    PartitionMetrics {
        name: partition.name,
        device: partition.device,
        mount_point: partition.mount_point,
        fs_type: partition.fs_type,
        total_bytes: partition.total_bytes as u64,
        used_bytes: partition.used_bytes as u64,
        available_bytes: partition.available_bytes as u64,
        used_blocks: partition.used_blocks as u64,
        available_blocks: partition.available_blocks as u64,
        block_size: partition.block_size as u64,
        collected_at: Some(to_timestamp(partition.collected_at)),
    }
}

pub fn partition_entry(
    row: (partition_entry::Model, Vec<partition_stats::Model>),
) -> PartitionEntry {
    let (entry, partitions) = row;
    PartitionEntry {
        partitions: partitions.into_iter().map(partition_metrics).collect(),
        collected_at: Some(to_timestamp(entry.collected_at)),
    }
}

pub fn network_metrics(network: network_stats::Model) -> NetworkMetrics {
    NetworkMetrics {
        local_ip: network.local_ip,
        total_bytes_transmitted: network.total_bytes_transmitted as u64,
        total_bytes_received: network.total_bytes_received as u64,
        total_packets_transmitted: network.total_packets_transmitted as u64,
        total_packets_received: network.total_packets_received as u64,
        collected_at: Some(to_timestamp(network.collected_at)),
    }
}

pub fn system_metrics(system: system_stats::Model) -> SystemMetrics {
    SystemMetrics {
        os_name: system.os_name,
        uptime_seconds: system.uptime_seconds as u64,
        host_name: system.host_name,
        kernel_version: system.kernel_version,
        collected_at: Some(to_timestamp(system.collected_at)),
    }
}

pub fn container_runtime_stats(
    row: (container_runtime_stats::Model, Vec<container_stats::Model>),
) -> ContainerRuntimeMetrics {
    let (runtime, containers) = row;
    ContainerRuntimeMetrics {
        containers: containers
            .into_iter()
            .map(|container| ContainerMetrics {
                container_runtime: container.container_runtime,
                container_id: container.container_id,
                host_name: container.host_name,
                created_at: container.created_at,
                status: container.status,
                running: container.running,
                running_for_seconds: container.running_for_seconds as u64,
                image_name: container.image_name,
                networks: container.networks.split('|').map(str::to_string).collect(),
                cpu_usage_percent: container.cpu_usage_percent,
                memory_usage_bytes: container.memory_usage_bytes as u64,
                collected_at: Some(to_timestamp(container.collected_at)),
            })
            .collect(),
        collected_at: Some(to_timestamp(runtime.collected_at)),
    }
}

pub fn speedtest_metrics(speedtest: speedtest_stats::Model) -> SpeedtestMetrics {
    SpeedtestMetrics {
        download_mbps: speedtest.download_mbps,
        upload_mbps: speedtest.upload_mbps,
        ping_ms: speedtest.ping_ms,
        collected_at: Some(to_timestamp(speedtest.collected_at)),
    }
}

pub fn error_stats_metrics(error: error::Model) -> ErrorStatsMetrics {
    ErrorStatsMetrics {
        file: error.file,
        line: error.line as u64,
        severity: error.severity,
        message: error.message,
        collected_at: Some(to_timestamp(error.collected_at)),
    }
}

fn process_stats_kind(kind: &str) -> ProcessStatsKind {
    match kind {
        "cpu" => ProcessStatsKind::Cpu,
        "memory" => ProcessStatsKind::Memory,
        _ => ProcessStatsKind::Unspecified,
    }
}

fn process_stats(process: process_stats::Model) -> ProcessStats {
    ProcessStats {
        kind: process_stats_kind(&process.kind) as i32,
        pid: process.pid as u32,
        name: process.name,
        user_name: process.user_name,
        status: process.status,
        cpu_usage_percent: process.cpu_usage_percent,
        memory_usage_bytes: process.memory_usage_bytes as u64,
    }
}

pub fn processes_stats(
    row: (processes_stats::Model, Vec<process_stats::Model>),
) -> ProcessesStats {
    let (stats, processes) = row;
    let (top_cpu, top_memory): (Vec<_>, Vec<_>) = processes
        .into_iter()
        .map(process_stats)
        .partition(|p| p.kind == ProcessStatsKind::Cpu as i32);

    ProcessesStats {
        top_cpu,
        top_memory,
        collected_at: Some(to_timestamp(stats.collected_at)),
    }
}

pub fn tunnel_access_log_metrics(log: tunnel_access_log::Model) -> TunnelAccessLogMetrics {
    TunnelAccessLogMetrics {
        sent_data: log.sent_data.to_string(),
        sent_at: Some(to_timestamp(log.sent_at)),
    }
}
