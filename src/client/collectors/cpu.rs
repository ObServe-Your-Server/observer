use std::time::Duration;
use sysinfo::{Components, System};

#[derive(Debug)]
pub struct CpuInfo {
    pub usage_percent: f32,
    pub count: usize,
    pub name: String,
    pub temp_celsius: Option<f32>,
}

pub fn collect(sys: &System) -> CpuInfo {
    let count = sys.cpus().len();
    let usage_percent =
        sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / count as f32;
    let name = sys
        .cpus()
        .first()
        .map(|c| c.brand())
        .unwrap_or_default()
        .to_string();

    // AMD: "k10temp Tctl", Intel: "coretemp Package id 0"
    let components = Components::new_with_refreshed_list();
    let temp_celsius = components
        .iter()
        .find(|c| {
            let label = c.label();
            label.contains("Tctl")
                || (label.starts_with("coretemp") && label.contains("Package id 0"))
        })
        .and_then(|c| c.temperature());

    CpuInfo {
        usage_percent,
        count,
        name,
        temp_celsius,
    }
}

/// Returns a System with two CPU refreshes separated by the minimum interval,
/// so `cpu_usage()` values are valid.
pub fn new_sys_with_cpu() -> System {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL + Duration::from_millis(300));
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    sys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_usage_in_valid_range() {
        let sys = new_sys_with_cpu();
        let cpu = collect(&sys);
        assert!(
            cpu.usage_percent >= 0.0 && cpu.usage_percent <= 100.0,
            "CPU usage out of range: {}",
            cpu.usage_percent
        );
    }
}
