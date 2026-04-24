use crate::mapper::host_metrics_models::speed_test_result::SpeedtestResult;
use crate::{
    mapper::host_metrics_models::{
        disk_info::DiskInfo, mapped_host_system_metrics::MappedHostSystemMetrics,
    },
    subsystem::host_metrics_collector::HostMetrics,
    subsystem::speedtest::SpeedtestMetrics,
};

pub struct HostSystemMapper {}

impl HostSystemMapper {
    pub fn map_for_watch_tower(
        current: HostMetrics,
        last: Option<HostMetrics>,
        speedtest: Option<SpeedtestMetrics>,
    ) -> MappedHostSystemMetrics {
        let cpu = current.cpu.as_ref();
        let memory = current.memory.as_ref();
        let network = current.network.as_ref();
        let system = current.system.as_ref();

        let net_bytes_received = network.map(|n| n.total_bytes_received).unwrap_or(0);
        let net_bytes_transmitted = network.map(|n| n.total_bytes_transmitted).unwrap_or(0);

        let (net_bytes_received_per_second, net_bytes_transmitted_per_second) =
            match last.as_ref().and_then(|l| l.network.as_ref()) {
                Some(last_net) => (
                    net_bytes_received.saturating_sub(last_net.total_bytes_received),
                    net_bytes_transmitted.saturating_sub(last_net.total_bytes_transmitted),
                ),
                None => (0, 0),
            };

        let disks = current
            .disks
            .unwrap_or_default()
            .into_iter()
            .map(|d| DiskInfo {
                name: d.name,
                total_bytes: d.total_bytes,
                used_bytes: d.used_bytes,
            })
            .collect();

        MappedHostSystemMetrics {
            cpu_usage_percent: cpu.map(|c| c.cpu_usage_percent).unwrap_or(0.0),
            cpu_count: cpu.map(|c| c.cpu_count as usize).unwrap_or(0),
            cpu_name: cpu.map(|c| c.cpu_name.clone()).unwrap_or_default(),
            ram_used_bytes: memory.map(|m| m.used_memory_in_byte).unwrap_or(0),
            ram_total_bytes: memory.map(|m| m.total_memory_in_byte).unwrap_or(0),
            uptime_secs: sysinfo::System::uptime(),
            cpu_temp_celsius: cpu.map(|c| c.cpu_temperature_celsius).filter(|&t| t != 0.0),
            os_name: system.and_then(|s| s.os_name.clone()),
            kernel_version: system.map(|s| s.kernel_version.clone()),
            net_bytes_received,
            net_bytes_transmitted,
            net_bytes_received_per_second,
            net_bytes_transmitted_per_second,
            local_ip: network.map(|n| n.local_ip.clone()),
            disks,
            speedtest_result: speedtest.map(|s| SpeedtestResult {
                download_mbps: s.download_mbps.unwrap_or(0.0),
                upload_mbps: s.upload_mbps.unwrap_or(0.0),
                ping_ms: s.ping_ms.unwrap_or(0.0),
            }),
            hostname: system.and_then(|s| s.host_name.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use open_eye::collector::{
        cpu::collector::{Core, CpuStats},
        disk::collector::DiskInfo as CollectorDiskInfo,
        memory::collector::MemoryStats,
        network::collector::NetworkStats,
        systemstats::collector::SystemStats,
    };

    use crate::subsystem::host_metrics_collector::HostMetrics;

    use super::HostSystemMapper;

    fn make_full_metrics() -> HostMetrics {
        HostMetrics {
            cpu: Some(CpuStats {
                cpu_name: "Test CPU".to_string(),
                cpu_count: 8,
                cpu_physical_count: 4,
                cpu_usage_percent: 42.5,
                cpu_temperature_celsius: 65.0,
                core_information: vec![Core {
                    core_name: "Core 0".to_string(),
                    core_usage_percent: 42.5,
                    core_frequency_mhz: 3200,
                }],
            }),
            memory: Some(MemoryStats {
                total_memory_in_byte: 16_000_000_000,
                available_memory_in_byte: 8_000_000_000,
                used_memory_in_byte: 8_000_000_000,
                total_swap_in_byte: 4_000_000_000,
                available_swap_in_byte: 4_000_000_000,
                used_swap_in_byte: 0,
            }),
            disks: Some(vec![CollectorDiskInfo {
                name: "sda".to_string(),
                total_bytes: 500_000_000_000,
                used_bytes: 200_000_000_000,
                available_bytes: 300_000_000_000,
                used_blocks: 390_625_000,
                available_blocks: 585_937_500,
                block_size: 512,
            }]),
            network: Some(NetworkStats {
                local_ip: "192.168.1.100".to_string(),
                total_bytes_transmitted: 5_000_000,
                total_bytes_received: 10_000_000,
                total_packets_transmitted: 5000,
                total_packets_received: 10000,
            }),
            system: Some(SystemStats {
                os_name: Some("Linux 6.1".to_string()),
                uptime_seconds: 1000,
                host_name: Some("test-host".to_string()),
                kernel_version: "6.1.0".to_string(),
            }),
        }
    }

    #[test]
    fn maps_full_metrics_correctly() {
        let last = HostMetrics {
            cpu: None,
            memory: None,
            disks: None,
            network: Some(NetworkStats {
                local_ip: "192.168.1.100".to_string(),
                total_bytes_transmitted: 4_000_000,
                total_bytes_received: 9_000_000,
                total_packets_transmitted: 4000,
                total_packets_received: 9000,
            }),
            system: None,
        };

        let result = HostSystemMapper::map_for_watch_tower(make_full_metrics(), Some(last), None);

        assert_eq!(result.cpu_name, "Test CPU");
        assert_eq!(result.cpu_count, 8);
        assert!((result.cpu_usage_percent - 42.5).abs() < f32::EPSILON);
        assert_eq!(result.cpu_temp_celsius, Some(65.0));
        assert_eq!(result.ram_used_bytes, 8_000_000_000);
        assert_eq!(result.ram_total_bytes, 16_000_000_000);
        assert_eq!(result.net_bytes_received, 10_000_000);
        assert_eq!(result.net_bytes_transmitted, 5_000_000);
        assert_eq!(result.net_bytes_received_per_second, 1_000_000);
        assert_eq!(result.net_bytes_transmitted_per_second, 1_000_000);
        assert_eq!(result.local_ip, Some("192.168.1.100".to_string()));
        assert_eq!(result.os_name, Some("Linux 6.1".to_string()));
        assert_eq!(result.kernel_version, Some("6.1.0".to_string()));
        assert_eq!(result.hostname, Some("test-host".to_string()));
        assert_eq!(result.disks.len(), 1);
        assert_eq!(result.disks[0].name, "sda");
        assert_eq!(result.disks[0].total_bytes, 500_000_000_000);
        assert_eq!(result.disks[0].used_bytes, 200_000_000_000);
        assert!(result.speedtest_result.is_none());
    }

    #[test]
    fn maps_all_none_metrics_to_zero_defaults() {
        let empty = HostMetrics {
            cpu: None,
            memory: None,
            disks: None,
            network: None,
            system: None,
        };

        let result = HostSystemMapper::map_for_watch_tower(empty, None, None);

        assert_eq!(result.cpu_name, "");
        assert_eq!(result.cpu_count, 0);
        assert!((result.cpu_usage_percent - 0.0).abs() < f32::EPSILON);
        assert_eq!(result.cpu_temp_celsius, None);
        assert_eq!(result.ram_used_bytes, 0);
        assert_eq!(result.ram_total_bytes, 0);
        assert_eq!(result.net_bytes_received, 0);
        assert_eq!(result.net_bytes_transmitted, 0);
        assert_eq!(result.net_bytes_received_per_second, 0);
        assert_eq!(result.net_bytes_transmitted_per_second, 0);
        assert_eq!(result.local_ip, None);
        assert_eq!(result.os_name, None);
        assert_eq!(result.kernel_version, None);
        assert_eq!(result.hostname, None);
        assert!(result.disks.is_empty());
        assert!(result.speedtest_result.is_none());
    }

    #[test]
    fn maps_speedtest_result_when_present() {
        use crate::subsystem::speedtest::SpeedtestMetrics;

        let speedtest = SpeedtestMetrics {
            download_mbps: Some(100.0),
            upload_mbps: Some(50.0),
            ping_ms: Some(12.0),
        };

        let empty = HostMetrics {
            cpu: None,
            memory: None,
            disks: None,
            network: None,
            system: None,
        };

        let result = HostSystemMapper::map_for_watch_tower(empty, None, Some(speedtest));

        let st = result
            .speedtest_result
            .expect("speedtest_result should be Some");
        assert!((st.download_mbps - 100.0).abs() < f64::EPSILON);
        assert!((st.upload_mbps - 50.0).abs() < f64::EPSILON);
        assert!((st.ping_ms - 12.0).abs() < f64::EPSILON);
    }
}
