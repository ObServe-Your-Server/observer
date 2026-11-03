use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, UdpSocket};
use sysinfo::{NetworkData, Networks};

pub const LOCAL_IP_UNKNOWN: &str = "not-found";

///TEST-NET-3 (RFC 5737)
const ROUTE_PROBE_ADDR: &str = "203.0.113.1:80";

///resolves the address of the interface owning the default route
///
///a connected udp socket makes the kernel apply its own routing decision, which
///is the same interface real traffic leaves by. This avoids guessing from
///interface names: en*/eth*
fn route_probe_addr() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect(ROUTE_PROBE_ADDR).ok()?;
    Some(socket.local_addr().ok()?.ip())
}

fn interface_owning(networks: &Networks, addr: IpAddr) -> Option<&NetworkData> {
    networks
        .iter()
        .find(|(_, n)| n.ip_networks().iter().any(|ip| ip.addr == addr))
        .map(|(_, n)| n)
}

fn is_private(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => {
            v4.is_private() || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
        }
        IpAddr::V6(v6) => {
            let s = v6.segments();
            (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Fallback for when no default route exists (offline host, isolated netns)
/// prefers private IPv4, then private IPv6, breaking ties on interface name
fn scan_interfaces_for_ip(networks: &Networks) -> Option<IpAddr> {
    let mut candidates: Vec<(&str, IpAddr)> = networks
        .iter()
        .flat_map(|(name, n)| {
            n.ip_networks()
                .iter()
                .map(move |ip| (name.as_str(), ip.addr))
        })
        .filter(|(_, addr)| {
            !addr.is_loopback()
                && !addr.is_unspecified()
                && !matches!(addr, IpAddr::V4(v4) if v4.is_link_local())
                && is_private(addr)
        })
        .collect();
    candidates.sort_by_key(|(name, addr)| (!addr.is_ipv4(), *name));
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
        let route_addr = route_probe_addr();
        let local_ip = route_addr
            .filter(is_private)
            .or_else(|| scan_interfaces_for_ip(&networks));

        //the unfiltered route address is used so hosts with a public ip on the uplink still report traffic
        let interface = [route_addr, local_ip]
            .into_iter()
            .flatten()
            .find_map(|addr| interface_owning(&networks, addr));

        let total_bytes_transmitted = interface.map_or(0, |n| n.total_transmitted());
        let total_bytes_received = interface.map_or(0, |n| n.total_received());
        let total_packets_transmitted = interface.map_or(0, |n| n.total_packets_transmitted());
        let total_packets_received = interface.map_or(0, |n| n.total_packets_received());

        let local_ip = local_ip
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
        interface_owning, is_private, route_probe_addr, scan_interfaces_for_ip, NetworkStats,
        LOCAL_IP_UNKNOWN,
    };
    use std::net::IpAddr;

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
        if route_probe_addr().filter(is_private).is_none()
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
        let parsed: IpAddr = ip.parse().expect("not a parseable address");
        assert!(is_private(&parsed), "resolved a non-local address: {ip}");
    }

    #[test]
    fn traffic_is_counted_on_route_interface() {
        let networks = sysinfo::Networks::new_with_refreshed_list();
        let Some(addr) = route_probe_addr() else {
            return;
        };
        if interface_owning(&networks, addr).is_none() {
            return;
        }
        let stats = NetworkStats::get_current_stats();
        assert!(
            stats.total_bytes_received > 0,
            "no bytes received on route interface"
        );
        assert!(
            stats.total_packets_received > 0,
            "no packets received on route interface"
        );
    }

    #[test]
    fn only_local_addresses_are_accepted() {
        for local in [
            "10.0.0.5",
            "192.168.1.20",
            "172.16.4.1",
            "100.100.0.1",
            "fd12:3456::1",
            "fe80::1",
        ] {
            assert!(
                is_private(&local.parse().unwrap()),
                "{local} should be local"
            );
        }
        for public in ["8.8.8.8", "203.0.113.1", "172.32.0.1", "2606:4700::1111"] {
            assert!(
                !is_private(&public.parse().unwrap()),
                "{public} should not be local"
            );
        }
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
