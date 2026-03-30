use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use sysinfo::Networks;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkStats {
    pub local_ip: String,
    pub total_bytes_transmitted: u64,
    pub total_bytes_received: u64,
    pub total_packets_transmitted: u64,
    pub total_packets_received: u64,
}

impl NetworkStats {
    /// Returns a snapshot of current network statistics.
    ///
    /// # Example
    ///
    /// ```
    /// use open_eye::collector::network::collector::NetworkStats;
    ///
    /// let stats = NetworkStats::get_current_stats();
    /// assert!(!stats.local_ip.is_empty());
    /// ```
    pub fn get_current_stats() -> NetworkStats {
        let networks = Networks::new_with_refreshed_list();
        let is_physical = |name: &String| {
            name.starts_with("eth") || name.starts_with("en") || name.starts_with("wl")
        };

        let total_bytes_transmitted = networks
            .iter()
            .filter(|(name, _)| is_physical(name))
            .map(|(_, n)| n.total_transmitted())
            .sum::<u64>();

        let total_bytes_received = networks
            .iter()
            .filter(|(name, _)| is_physical(name))
            .map(|(_, n)| n.total_received())
            .sum::<u64>();

        let total_packets_transmitted = networks
            .iter()
            .filter(|(name, _)| is_physical(name))
            .map(|(_, n)| n.total_packets_transmitted())
            .sum::<u64>();

        let total_packets_received = networks
            .iter()
            .filter(|(name, _)| is_physical(name))
            .map(|(_, n)| n.total_packets_transmitted())
            .sum::<u64>();

        let local_ip = match local_ip() {
            Ok(ip) => ip.to_string(),
            Err(err) => {
                log::debug!("Error during gathering of local ip {}", err);
                "not-found".to_string()
            }
        };

        NetworkStats {
            local_ip,
            total_bytes_transmitted,
            total_bytes_received,
            total_packets_transmitted,
            total_packets_received,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::collector::network::collector::NetworkStats;

    #[test]
    fn get_current_network_stats_test() {
        let first = NetworkStats::get_current_stats();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let second = NetworkStats::get_current_stats();

        assert!(!first.local_ip.is_empty(), "local_ip should not be empty");
        assert!(
            second.total_bytes_received >= first.total_bytes_received,
            "bytes_received should not decrease"
        );
        assert!(
            second.total_bytes_transmitted >= first.total_bytes_transmitted,
            "bytes_transmitted should not decrease"
        );
    }
}
