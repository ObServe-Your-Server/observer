//! Converts SeaORM entity models (as stored in the DB) into the proto metric types.

use crate::entities::{
    container_runtime_stats, container_stats, cpu_core_stats, cpu_stats, disk_stats,
    memory_stats, network_stats, process_stats, processes_stats, speedtest_stats, system_stats,
};
use crate::grpc::v1::metrics::{
    ContainerRuntimeStats, ContainerStats, CoreMetrics, CpuMetrics, DiskMetrics, MemoryMetrics,
    NetworkMetrics, ProcessStats, ProcessStatsKind, ProcessesStats, SpeedtestMetrics,
    SystemMetrics,
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

pub fn disk_metrics(disk: disk_stats::Model) -> DiskMetrics {
    DiskMetrics {
        name: disk.name,
        total_bytes: disk.total_bytes as u64,
        used_bytes: disk.used_bytes as u64,
        used_blocks: disk.used_blocks as u64,
        available_bytes: disk.available_bytes as u64,
        available_blocks: disk.available_blocks as u64,
        block_size: disk.block_size as u64,
        collected_at: Some(to_timestamp(disk.collected_at)),
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

fn process_stats_kind(kind: &str) -> ProcessStatsKind {
    match kind {
        "cpu" => ProcessStatsKind::Cpu,
        "memory" => ProcessStatsKind::Memory,
        _ => ProcessStatsKind::Unspecified,
    }
}

pub fn processes_stats(
    row: (processes_stats::Model, Vec<process_stats::Model>),
) -> ProcessesStats {
    let (processes, process_rows) = row;
    let (top_cpu, top_memory): (Vec<_>, Vec<_>) = process_rows
        .into_iter()
        .partition(|process| process.kind == "cpu");

    let map_process = |process: process_stats::Model| ProcessStats {
        kind: process_stats_kind(&process.kind) as i32,
        pid: process.pid as u32,
        name: process.name,
        user_name: process.user_name,
        status: process.status,
        cpu_usage_percent: process.cpu_usage_percent,
        memory_usage_bytes: process.memory_usage_bytes as u64,
    };

    ProcessesStats {
        top_cpu: top_cpu.into_iter().map(map_process).collect(),
        top_memory: top_memory.into_iter().map(map_process).collect(),
        collected_at: Some(to_timestamp(processes.collected_at)),
    }
}

pub fn container_runtime_stats(
    row: (
        container_runtime_stats::Model,
        Vec<container_stats::Model>,
    ),
) -> ContainerRuntimeStats {
    let (runtime, containers) = row;
    ContainerRuntimeStats {
        containers: containers
            .into_iter()
            .map(|container| ContainerStats {
                container_runtime: container.container_runtime,
                id: container.container_id,
                host_name: container.host_name,
                created_at: container.created_at,
                status: container.status,
                running: container.running,
                running_for_seconds: container.running_for_seconds as u64,
                image_name: container.image_name,
                networks: container.networks.split(',').map(str::to_string).collect(),
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
