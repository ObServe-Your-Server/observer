use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, UdpSocket};
use sysinfo::Networks;

pub const LOCAL_IP_UNKNOWN: &str = "not-found";

///TEST-NET-3 (RFC 5737)
const ROUTE_PROBE_ADDR: &str = "203.0.113.1:80";

///resolves the address of the interface owning the default route
///
///a connected udp socket makes the kernel apply its own routing decision, which
///is the same interface real traffic leaves by. This avoids guessing from
///interface names: en*/eth*
fn default_route_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect(ROUTE_PROBE_ADDR).ok()?;
    let addr = socket.local_addr().ok()?.ip();
    (!addr.is_loopback() && !addr.is_unspecified()).then_some(addr)
}

/// Fallback for when no default route exists (offline host, isolated netns)
/// picks the lowest-sorted interface name
fn scan_interfaces_for_ip(networks: &Networks) -> Option<IpAddr> {
    let mut candidates: Vec<(&str, IpAddr)> = networks
        .iter()
        .flat_map(|(name, n)| {
            n.ip_networks()
                .iter()
                .map(move |ip| (name.as_str(), ip.addr))
        })
        .filter(|(_, addr)| {
            addr.is_ipv4()
                && !addr.is_loopback()
                && !addr.is_unspecified()
                && !matches!(addr, IpAddr::V4(v4) if v4.is_link_local())
        })
        .collect();
    candidates.sort_by_key(|(name, _)| *name);
    candidates.first().map(|(_, addr)| *addr)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkStats {
    pub local_ip: String,
    pub total_bytes_transmitted: u64,
    pub total_bytes_received: u64,
    pub total_packets_transmitted: u64,
    pub total_packets_received: u64,
    pub collected_at: chrono::DateTime<Utc>,
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
            .map(|(_, n)| n.total_packets_received())
            .sum::<u64>();

        let local_ip = default_route_ip()
            .or_else(|| scan_interfaces_for_ip(&networks))
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| LOCAL_IP_UNKNOWN.to_string());

        NetworkStats {
            local_ip,
            total_bytes_transmitted,
            total_bytes_received,
            total_packets_transmitted,
            total_packets_received,
            collected_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::collector::network::collector::{
        default_route_ip, scan_interfaces_for_ip, NetworkStats, LOCAL_IP_UNKNOWN,
    };

    #[test]
    fn local_ip_is_stable_across_collections() {
        //the old name-prefix selection read from a hash map and could return a
        //different interface each call
        let first = NetworkStats::get_current_stats().local_ip;
        for _ in 0..10 {
            assert_eq!(
                first,
                NetworkStats::get_current_stats().local_ip,
                "local_ip changed between collections"
            );
        }
    }

    #[test]
    fn local_ip_is_not_loopback_or_placeholder() {
        // Skip rather than fail where the sandbox genuinely has no route.
        if default_route_ip().is_none()
            && scan_interfaces_for_ip(&sysinfo::Networks::new_with_refreshed_list()).is_none()
        {
            return;
        }
        let ip = NetworkStats::get_current_stats().local_ip;
        assert_ne!(ip, LOCAL_IP_UNKNOWN, "no address resolved");
        assert!(
            !ip.starts_with("127.") && ip != "0.0.0.0",
            "resolved a loopback/unspecified address: {ip}"
        );
        assert!(
            ip.parse::<std::net::IpAddr>().is_ok(),
            "not a parseable address: {ip}"
        );
    }

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
