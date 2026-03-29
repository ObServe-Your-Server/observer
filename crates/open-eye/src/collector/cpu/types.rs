
#[derive(Debug, Clone, PartialEq)]
pub struct CpuStats {
    pub cpu_name: String,
    pub cpu_count: u8,
    pub cpu_physical_count: u8,
    pub cpu_usage: u8,
    pub cpu_temperature: i16,
    pub core_infos: Vec<Core>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Core {
    pub core_name: String,
    pub core_usage: u8,
    pub core_frequency: u16,
}



#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sysinfo::{Components, Disks, Motherboard, System};



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
        
        let usage_percent = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;
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

