use std::time::Duration;

use sysinfo::{Components, System};

#[derive(Debug, Clone, PartialEq)]
pub struct CpuStats {
    pub cpu_name: String,
    pub cpu_count: u16,
    pub cpu_physical_count: u16,
    pub cpu_usage_percent: f32,
    pub cpu_temperature_celsius: f32,
    pub core_infos: Vec<Core>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Core {
    pub core_name: String,
    pub core_usage_percent: f32,
    pub core_frequency_mhz: u64,
}

pub fn get_current_cpu_stats() -> CpuStats {
    let mut sys = System::new_all();
    sys.refresh_cpu_all();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL + Duration::from_millis(200));
    sys.refresh_cpu_all();

    let cpu_name = match sys.cpus().first() {
        Some(c) => c.brand().to_string(),
        None => "Not found".to_string(),
    };
    let cpu_count = sys.cpus().iter().count() as u16;
    let cpu_physical_count = System::physical_core_count().get_or_insert(0).clone() as u16;
    let cpu_usage_percent =
        sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;
    let cpu_temperature_celsius = get_cpu_temperature().get_or_insert(0.0).clone();
    let core_infos = get_core_infos(&sys);

    CpuStats {
        cpu_name,
        cpu_count,
        cpu_physical_count,
        cpu_usage_percent,
        cpu_temperature_celsius,
        core_infos,
    }
}

fn get_core_infos(sys: &System) -> Vec<Core> {
    let mut res: Vec<Core> = Vec::new();
    for c in sys.cpus() {
        res.push(Core {
            core_name: c.brand().to_string(),
            core_usage_percent: c.cpu_usage(),
            core_frequency_mhz: c.frequency(),
        });
    }
    return res;
}

fn get_cpu_temperature() -> Option<f32> {
    let components = Components::new_with_refreshed_list();
    components
        .iter()
        .find(|c| {
            let label = c.label();
            label.contains("Tctl")
                || (label.starts_with("coretemp") && label.contains("Package id 0"))
        })
        .and_then(|c| c.temperature())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sysinfo::{Components, Disks, Motherboard, System};

    use crate::collector::cpu::types::get_current_cpu_stats;

    #[test]
    fn get_current_cpu_stats_test() {
        let res: crate::collector::cpu::types::CpuStats = get_current_cpu_stats();

        assert!(!res.cpu_name.is_empty(), "cpu_name should not be empty");
        assert!(res.cpu_count > 0, "cpu_count should be > 0");
        assert!(
            res.cpu_physical_count > 0,
            "cpu_physical_count should be > 0"
        );
        assert!(
            res.cpu_physical_count <= res.cpu_count,
            "physical cores ({}) should be <= logical cores ({})",
            res.cpu_physical_count,
            res.cpu_count
        );
        assert!(
            res.cpu_usage_percent >= 0.0 && res.cpu_usage_percent <= 100.0,
            "cpu_usage_percent {} should be in [0, 100]",
            res.cpu_usage_percent
        );
        assert!(
            res.core_infos.len() == res.cpu_count as usize,
            "core_infos length {} should equal cpu_count {}",
            res.core_infos.len(),
            res.cpu_count
        );
        for core in &res.core_infos {
            assert!(!core.core_name.is_empty(), "core_name should not be empty");
            assert!(
                core.core_usage_percent >= 0.0 && core.core_usage_percent <= 100.0,
                "core_usage_percent {} should be in [0, 100]",
                core.core_usage_percent
            );
            assert!(
                core.core_frequency_mhz > 0,
                "core_frequency_mhz should be > 0"
            );
        }
    }

    #[test]
    fn all_output() {
        let mut sys = System::new_all();
        sys.refresh_all();

        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL + Duration::from_millis(200));

        sys.refresh_all();

        println!("{:?}", sys);

        println!("cpus: {:?}", sys.cpus());
        println!("cgroup limits: {:?}", sys.cgroup_limits());
        //println!("{:?}", sys.processes());

        for cpu in sys.cpus() {
            println!("frequency: {}", cpu.frequency());
        }

        println!("threads: {}", sys.cpus().iter().count());
        println!("physical core count: {:?}", System::physical_core_count());

        let usage_percent =
            sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;
        println!("cpu usage: {}", usage_percent);

        let components = Components::new_with_refreshed_list();
        let temp_celsius = components
            .iter()
            .find(|c| {
                let label = c.label();
                label.contains("Tctl")
                    || (label.starts_with("coretemp") && label.contains("Package id 0"))
            })
            .and_then(|c| c.temperature());

        if let Some(temp) = temp_celsius {
            println!("cpu temp: {}°C", temp);
        }

        println!("process count: {:?}", sys.processes().into_iter().count());

        println!("boot time: {:?}", System::boot_time());

        println!("host name: {:?}", System::host_name());
        println!("load average: {:?}", System::load_average());
        println!("system name: {:?}", System::name());

        let motherboard = Motherboard::new().unwrap();
        println!("{:?}", motherboard.vendor_name());

        println!("");

        let disk_info = Disks::new_with_refreshed_list();
        for disk in disk_info.into_iter() {
            println!("{:?}", disk.name());
            println!("{:?}", disk.total_space());
            println!("{:?}", disk.available_space());
            println!("")
        }
    }
}
