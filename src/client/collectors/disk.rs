use sysinfo::Disks;

#[derive(Debug)]
pub struct DiskInfo {
    pub name: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
}

const VIRTUAL_FS: &[&str] = &[
    "tmpfs", "devtmpfs", "squashfs", "overlay", "sysfs", "proc", "cgroup", "cgroup2",
    "devpts", "securityfs", "pstore", "efivarfs", "bpf", "autofs", "hugetlbfs",
    "mqueue", "debugfs", "tracefs", "fusectl", "configfs", "ramfs",
];

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

pub fn collect() -> Vec<DiskInfo> {
    let sysinfo_disks = Disks::new_with_refreshed_list();
    let mut best: std::collections::HashMap<String, (u64, u64)> =
        std::collections::HashMap::new();

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
