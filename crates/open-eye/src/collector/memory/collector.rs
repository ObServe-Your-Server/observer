use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_memory_in_byte: u64,
    pub available_memory_in_byte: u64,
    pub used_memory_in_byte: u64,
    pub total_swap_in_byte: u64,
    pub available_swap_in_byte: u64,
    pub used_swap_in_byte: u64,
}

impl MemoryStats {
    pub fn get_current_stats() -> MemoryStats {
        let mut sys = System::new_all();
        sys.refresh_memory();

        MemoryStats {
            total_memory_in_byte: sys.total_memory(),
            available_memory_in_byte: sys.available_memory(),
            used_memory_in_byte: sys.used_memory(),
            total_swap_in_byte: sys.total_swap(),
            available_swap_in_byte: sys.free_swap(),
            used_swap_in_byte: sys.used_swap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sysinfo::System;

    use crate::collector::memory::collector::MemoryStats;

    #[test]
    fn get_current_memory_stats_test() {
        let res = MemoryStats::get_current_stats();

        assert!(res.total_memory_in_byte > 0, "total_memory should be > 0");
        assert!(
            res.available_memory_in_byte <= res.total_memory_in_byte,
            "available_memory ({}) should be <= total_memory ({})",
            res.available_memory_in_byte,
            res.total_memory_in_byte
        );
        assert!(
            res.used_memory_in_byte <= res.total_memory_in_byte,
            "used_memory ({}) should be <= total_memory ({})",
            res.used_memory_in_byte,
            res.total_memory_in_byte
        );
        assert!(
            res.available_swap_in_byte <= res.total_swap_in_byte,
            "available_swap ({}) should be <= total_swap ({})",
            res.available_swap_in_byte,
            res.total_swap_in_byte
        );
        assert!(
            res.used_swap_in_byte <= res.total_swap_in_byte,
            "used_swap ({}) should be <= total_swap ({})",
            res.used_swap_in_byte,
            res.total_swap_in_byte
        );
    }

    #[test]
    fn get_all_memory_stats_test() {
        let mut sys = System::new_all();
        sys.refresh_all();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL + Duration::from_millis(200));
        sys.refresh_all();

        println!("total memory: {:?}", sys.total_memory());
        println!("avaiable memory: {:?}", sys.available_memory());
        println!("used memory: {:?}", sys.used_memory());
        println!();
        println!("total swap: {:?}", sys.total_swap());
        println!("used swap: {:?}", sys.used_swap());
    }
}
