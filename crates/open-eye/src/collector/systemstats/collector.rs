use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemStats {
    pub os_name: Option<String>,
    pub host_name: Option<String>,
    pub kernel_version: String,
}

impl SystemStats {
    /// Returns a snapshot of current system statistics.
    ///
    /// # Example
    ///
    /// ```
    /// use open_eye::collector::systemstats::collector::SystemStats;
    ///
    /// let stats = SystemStats::get_current_stats();
    /// assert!(!stats.kernel_version.is_empty());
    /// ```
    pub fn get_current_stats() -> SystemStats {
        SystemStats {
            os_name: System::long_os_version(),
            host_name: System::host_name(),
            kernel_version: System::kernel_long_version(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{collector::systemstats::collector::SystemStats, logging::init_logging};

    #[test]
    fn get_current_system_stats_test() {
        let res = SystemStats::get_current_stats();

        assert!(!res.kernel_version.is_empty(), "kernel version is empty")
    }
}
