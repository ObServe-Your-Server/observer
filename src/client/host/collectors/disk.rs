#[cfg(not(target_os = "linux"))]
use sysinfo::Disks;

#[derive(Debug, Clone, serde::Serialize)]
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
    use log::{debug, warn};
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
        size: Option<u64>,
        #[serde(default)]
        fsused: Option<u64>,
        #[serde(default)]
        fsavail: Option<u64>,
        #[serde(default)]
        mountpoint: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(rename = "type", default)]
        r#type: Option<String>,
        #[serde(default)]
        children: Option<Vec<BlockDevice>>,
    }

    /// Only accept actual physical disks (TYPE=disk).
    /// This reliably excludes LVM volumes (dm-*), software RAID (md*),
    /// optical drives, loop devices, RAM disks, and any other virtual block device.
    fn is_physical_disk(dev: &BlockDevice) -> bool {
        dev.r#type.as_deref() == Some("disk")
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
            total += dev.fsused.unwrap_or(0);
        }
        if let Some(children) = &dev.children {
            for child in children {
                total += sum_used(child);
            }
        }
        total
    }

    pub fn collect() -> Vec<DiskInfo> {
        // -b: output sizes in exact bytes (no human-readable rounding)
        // TYPE column lets us filter to only real physical disks
        let output = Command::new("lsblk")
            .args(["-b", "-J", "-o", "NAME,SIZE,FSUSED,FSAVAIL,MOUNTPOINT,MODEL,TYPE"])
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

        // Log every top-level block device so we can see what is accepted/skipped.
        for dev in &parsed.blockdevices {
            let ty = dev.r#type.as_deref().unwrap_or("?");
            let size_gb = dev.size.unwrap_or(0) as f64 / 1024.0 / 1024.0 / 1024.0;
            let model = dev.model.as_deref().unwrap_or("-");
            debug!(
                "lsblk block device: /dev/{} type={} size={:.1}GB model={}",
                dev.name, ty, size_gb, model
            );
        }

        let mut disks: Vec<DiskInfo> = parsed
            .blockdevices
            .iter()
            .filter(|dev| {
                if !is_physical_disk(dev) {
                    debug!(
                        "  skipping /dev/{}: type={} (not a physical disk)",
                        dev.name,
                        dev.r#type.as_deref().unwrap_or("?")
                    );
                    return false;
                }
                let size = dev.size.unwrap_or(0);
                if size == 0 {
                    debug!(
                        "  skipping /dev/{}: size is 0 (uninitialized or virtual device)",
                        dev.name
                    );
                    return false;
                }
                true
            })
            .map(|dev| {
                let total_bytes = dev.size.unwrap_or(0);
                let used_bytes = sum_used(dev);
                let name = dev
                    .model
                    .as_deref()
                    .map(|m| m.trim())
                    .filter(|m| !m.is_empty())
                    .unwrap_or(&dev.name)
                    .to_string();

                debug!(
                    "  accepted /dev/{} as \"{}\": total={:.1}GB used={:.1}GB",
                    dev.name,
                    name,
                    total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                    used_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
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
