use log::info;
use serde::Serialize;
#[cfg(not(target_os = "linux"))]
use sysinfo::Disks;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
}

// ── Linux: lsblk-based collection ────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux {
    use super::DiskInfo;
    use log::{info, warn};
    use serde::Deserialize;
    use std::process::Command;

    #[derive(Debug, Deserialize)]
    struct LsblkOutput {
        blockdevices: Vec<BlockDevice>,
    }

    #[derive(Debug, Deserialize)]
    struct BlockDevice {
        name: String,
        #[serde(default)]
        size: Option<String>,
        #[serde(default)]
        fsused: Option<String>,
        #[serde(default)]
        fsavail: Option<String>,
        #[serde(default)]
        mountpoint: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        children: Option<Vec<BlockDevice>>,
    }

    /// Filters out optical drives (sr*), floppy (fd*), loop, ram, zram,
    /// and anything whose model name suggests DVD/CD/ROM.
    fn is_standard_device(dev: &BlockDevice) -> bool {
        let name = dev.name.as_str();
        if name.starts_with("sr")
            || name.starts_with("fd")
            || name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("zram")
        {
            return false;
        }
        if let Some(model) = &dev.model {
            let m = model.to_uppercase();
            if m.contains("DVD")
                || m.contains("CD-ROM")
                || m.contains("CD ROM")
                || m.contains("OPTICAL")
            {
                return false;
            }
        }
        true
    }

    /// Parse a human-readable size string from lsblk (e.g. "64G", "37.5G", "1M") into bytes.
    fn parse_size(s: &str) -> u64 {
        let s = s.trim();
        let split = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
        let num: f64 = s[..split].parse().unwrap_or(0.0);
        let unit = s[split..].to_uppercase();
        let multiplier: u64 = match unit.as_str() {
            "K" | "KIB" | "KB" => 1024,
            "M" | "MIB" | "MB" => 1024 * 1024,
            "G" | "GIB" | "GB" => 1024 * 1024 * 1024,
            "T" | "TIB" | "TB" => 1024_u64 * 1024 * 1024 * 1024,
            _ => 1,
        };
        (num * multiplier as f64) as u64
    }

    /// Recursively sum fsused across all mounted partitions of a device.
    fn sum_used(dev: &BlockDevice) -> u64 {
        let mut total: u64 = 0;
        if dev
            .mountpoint
            .as_deref()
            .map(|m| !m.is_empty())
            .unwrap_or(false)
        {
            total += dev.fsused.as_deref().map(parse_size).unwrap_or(0);
        }
        if let Some(children) = &dev.children {
            for child in children {
                total += sum_used(child);
            }
        }
        total
    }

    pub fn collect() -> Vec<DiskInfo> {
        let output = Command::new("lsblk")
            .args(["-J", "-o", "NAME,SIZE,FSUSED,FSAVAIL,MOUNTPOINT,MODEL"])
            .output();

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                warn!("lsblk failed to run: {}", e);
                return vec![];
            }
        };

        let json = match std::str::from_utf8(&output.stdout) {
            Ok(s) => s,
            Err(e) => {
                warn!("lsblk output is not valid UTF-8: {}", e);
                return vec![];
            }
        };

        let parsed: LsblkOutput = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to parse lsblk JSON: {}", e);
                return vec![];
            }
        };

        let mut disks: Vec<DiskInfo> = parsed
            .blockdevices
            .iter()
            .filter(|dev| is_standard_device(dev))
            .map(|dev| {
                let total_bytes = dev.size.as_deref().map(parse_size).unwrap_or(0);
                let used_bytes = sum_used(dev);
                let name = dev
                    .model
                    .as_deref()
                    .map(|m| m.trim())
                    .filter(|m| !m.is_empty())
                    .unwrap_or("Unknown")
                    .to_string();

                info!(
                    "lsblk disk: dev={} name={} total={}B used={}B",
                    dev.name, name, total_bytes, used_bytes,
                );

                DiskInfo {
                    name,
                    total_bytes,
                    used_bytes,
                }
            })
            .collect();

        disks.sort_by(|a, b| a.name.cmp(&b.name));
        disks
    }
}

// ── Non-Linux: sysinfo-based collection ──────────────────────────────────────

#[cfg(not(target_os = "linux"))]
const VIRTUAL_FS: &[&str] = &[
    "tmpfs",
    "devtmpfs",
    "squashfs",
    "overlay",
    "sysfs",
    "proc",
    "cgroup",
    "cgroup2",
    "devpts",
    "securityfs",
    "pstore",
    "efivarfs",
    "bpf",
    "autofs",
    "hugetlbfs",
    "mqueue",
    "debugfs",
    "tracefs",
    "fusectl",
    "configfs",
    "ramfs",
];

#[cfg(not(target_os = "linux"))]
fn is_real_disk(d: &sysinfo::Disk) -> bool {
    if d.total_space() == 0 {
        return false;
    }
    let name = d.name().to_string_lossy();
    let dev = name.rsplit('/').next().unwrap_or(name.as_ref());
    if dev.starts_with("loop") || dev.starts_with("ram") || dev.starts_with("zram") {
        return false;
    }
    let fs = d.file_system().to_string_lossy().to_lowercase();
    if VIRTUAL_FS.iter().any(|v| fs == *v) {
        return false;
    }
    true
}

/// Maps a partition device name to its parent disk.
/// e.g. "nvme0n1p2" → "nvme0n1", "sda1" → "sda", "mmcblk0p1" → "mmcblk0"
#[cfg(not(target_os = "linux"))]
fn parent_disk_name(dev: &str) -> String {
    if dev.starts_with("nvme") || dev.starts_with("mmcblk") {
        if let Some(pos) = dev.rfind('p') {
            let suffix = &dev[pos + 1..];
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                return dev[..pos].to_string();
            }
        }
    } else {
        let stripped = dev.trim_end_matches(|c: char| c.is_ascii_digit());
        if stripped.len() < dev.len() {
            return stripped.to_string();
        }
    }
    dev.to_string()
}

#[cfg(not(target_os = "linux"))]
pub fn collect() -> Vec<DiskInfo> {
    let sysinfo_disks = Disks::new_with_refreshed_list();
    let mut best: std::collections::HashMap<String, (u64, u64)> = std::collections::HashMap::new();

    for d in sysinfo_disks.iter().filter(|d| is_real_disk(d)) {
        let full = d.name().to_string_lossy();
        let dev = full.rsplit('/').next().unwrap_or(full.as_ref());
        let parent = parent_disk_name(dev);
        let used = d.total_space() - d.available_space();
        best.entry(parent)
            .and_modify(|(tot, use_)| {
                if d.total_space() > *tot {
                    *tot = d.total_space();
                    *use_ = used;
                }
            })
            .or_insert((d.total_space(), used));
    }

    let mut disks: Vec<DiskInfo> = best
        .into_iter()
        .map(|(name, (total_bytes, used_bytes))| DiskInfo {
            name,
            total_bytes,
            used_bytes,
        })
        .collect();
    disks.sort_by(|a, b| a.name.cmp(&b.name));
    disks
}

#[cfg(target_os = "linux")]
pub fn collect() -> Vec<DiskInfo> {
    linux::collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_used_does_not_exceed_total() {
        for disk in collect() {
            assert!(
                disk.total_bytes > 0,
                "Disk [{}] total should be greater than 0",
                disk.name
            );
            assert!(
                disk.used_bytes <= disk.total_bytes,
                "Disk [{}] used {}B exceeds total {}B",
                disk.name,
                disk.used_bytes,
                disk.total_bytes
            );
        }
    }
}
