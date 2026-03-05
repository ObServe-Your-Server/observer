use local_ip_address::local_ip;
use sysinfo::Networks;

#[derive(Debug)]
pub struct NetworkInfo {
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub local_ip: Option<String>,
}

impl NetworkInfo {
    pub fn local_ip() -> Option<String> {
        local_ip().ok().map(|ip| ip.to_string())
    }
}

pub fn collect() -> NetworkInfo {
    let networks = Networks::new_with_refreshed_list();
    let bytes_received = networks
        .iter()
        .filter(|(name, _)| *name != "lo")
        .map(|(_, n)| n.total_received())
        .sum();
    let bytes_transmitted = networks
        .iter()
        .filter(|(name, _)| *name != "lo")
        .map(|(_, n)| n.total_transmitted())
        .sum();

    NetworkInfo {
        bytes_received,
        bytes_transmitted,
        local_ip: NetworkInfo::local_ip(),
    }
}
