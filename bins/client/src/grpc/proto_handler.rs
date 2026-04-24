use crate::subsystem::host_metrics_collector::HostMetrics;
use super::metrics_proto::{Cpu, Core, Disk, FullMetrics, Memory, Network, SystemStats};

pub fn to_full_metrics(host: HostMetrics) -> FullMetrics {
    let now = prost_types::Timestamp::from(std::time::SystemTime::now());

    let cpu_vec = host.cpu.map(|c| Cpu {
        cpu_name: c.cpu_name,
        cpu_count: c.cpu_count as u32,
        cpu_physical_count: c.cpu_physical_count as u32,
        cpu_usage_percent: c.cpu_usage_percent as f64,
        cpu_temperature_celsius: c.cpu_temperature_celsius as f64,
        cores: c.core_information.into_iter().map(|core| Core {
            core_name: core.core_name,
            core_usage_percent: core.core_usage_percent as f64,
            core_frequency_mhz: core.core_frequency_mhz as f64,
            recorded_at: Some(now.clone()),
        }).collect(),
        recorded_at: Some(now.clone()),
    }).into_iter().collect();

    let memory = host.memory.map(|m| Memory {
        total_memory_in_byte: m.total_memory_in_byte,
        available_memory_in_byte: m.available_memory_in_byte,
        used_memory_in_byte: m.used_memory_in_byte,
        total_swap_in_byte: m.total_swap_in_byte,
        used_swap_in_byte: m.used_swap_in_byte,
        recorded_at: Some(now.clone()),
    });

    let disk = host.disks.unwrap_or_default().into_iter().map(|d| Disk {
        name: d.name,
        total_bytes: d.total_bytes,
        used_bytes: d.used_bytes,
        available_bytes: d.available_bytes,
        used_blocks: d.used_blocks,
        available_blocks: d.available_blocks,
        block_size: d.block_size,
        recorded_at: Some(now.clone()),
    }).collect();

    let network = host.network.map(|n| Network {
        local_ip: n.local_ip,
        total_bytes_transmitted: n.total_bytes_transmitted,
        total_bytes_received: n.total_bytes_received,
        total_packets_transmitted: n.total_packets_transmitted,
        total_packets_received: n.total_packets_received,
        recorded_at: Some(now.clone()),
    });

    let system_stats = host.system.map(|s| SystemStats {
        os_name: s.os_name,
        hostname: s.host_name,
        kernel_version: Some(s.kernel_version),
        uptime_in_seconds: Some(s.uptime_seconds),
        recorded_at: Some(now.clone()),
    });

    FullMetrics {
        metrics: cpu_vec,
        memory,
        disk,
        network,
        system_stats,
        recorded_at: Some(now),
    }
}
