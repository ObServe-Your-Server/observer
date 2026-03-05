use hardware_query::{HardwareInfo, MemoryInfo};

pub fn ram_information() -> Option<MemoryInfo> {
    let hw = HardwareInfo::query().ok()?;
    Some(hw.memory().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_information() {
        let info = ram_information().expect("ram information failed");
        println!("{:#?}", info);
        assert!(info.total_mb > 0, "total memory should be > 0");
    }

    #[test]
    fn test_print_all_hardware_info() {
        let hw = HardwareInfo::query().expect("hardware query failed");
        println!("{:#?}", hw);
    }
}
