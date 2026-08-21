use chrono::Utc;
use nix::sys::statvfs::statvfs;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartitionInfo {
    pub name: String,
    /// Source device path (e.g. "/dev/sda1"), or pool name for ZFS.
    pub device: String,
    /// Mount point of the partition (e.g. "/", "/home").
    pub mount_point: String,
    /// Filesystem type (e.g. "ext4", "apfs", "zfs").
    pub fs_type: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub used_blocks: u64,
    pub available_blocks: u64,
    pub block_size: u64,
    pub collected_at: chrono::DateTime<Utc>,
}

// ── Linux ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux {
    use super::{collect_zpools, PartitionInfo};
    use log::{debug, warn};
    use std::collections::HashSet;
    use std::fs;

    const SKIP_PREFIXES: &[&str] = &["/proc", "/sys", "/dev", "/run"];

    const VIRTUAL_FS: &[&str] = &[
        "proc",
        "sysfs",
        "devtmpfs",
        "devpts",
        "tmpfs",
        "cgroup",
        "cgroup2",
        "pstore",
        "bpf",
        "autofs",
        "mqueue",
        "debugfs",
        "tracefs",
        "securityfs",
        "configfs",
        "fusectl",
        "overlay",
        "squashfs",
        "nsfs",
        "binfmt_misc",
        "rpc_pipefs",
        "zfs",
    ];

    fn unescape_mount_field(field: &str) -> String {
        // /proc/mounts encodes space, tab, newline and backslash as \040, \011, \012, \134
        field
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\")
    }

    pub fn get_current_stats() -> Vec<PartitionInfo> {
        let contents = match fs::read_to_string("/proc/mounts") {
            Ok(c) => c,
            Err(e) => {
                warn!("failed to read /proc/mounts: {}", e);
                return vec![];
            }
        };

        let mut seen = HashSet::new();
        let mut disks = vec![];

        for line in contents.lines() {
            // Format: "device mountpoint fstype options dump pass"
            let mut parts = line.split_whitespace();
            let device = match parts.next() {
                Some(d) => d,
                None => continue,
            };
            let mount = match parts.next() {
                Some(m) => unescape_mount_field(m),
                None => continue,
            };
            let fstype = parts.next().unwrap_or("");

            if VIRTUAL_FS.iter().any(|v| fstype.eq_ignore_ascii_case(v)) {
                continue;
            }

            if SKIP_PREFIXES.iter().any(|p| mount.starts_with(p)) {
                continue;
            }

            // Deduplicate by device path (handles bind mounts of the same partition)
            if !seen.insert(device.to_string()) {
                continue;
            }

            if let Some(info) = super::statvfs_info(&mount, &mount, device, fstype) {
                if info.total_bytes == 0 {
                    continue;
                }
                debug!(
                    "Linux partition: {} at {} — {:.1}GB total",
                    device,
                    mount,
                    info.total_bytes as f64 / 1_073_741_824.0
                );
                disks.push(info);
            }
        }

        disks.extend(collect_zpools());
        disks.sort_by(|a, b| a.name.cmp(&b.name));
        disks
    }
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use super::{collect_zpools, PartitionInfo};
    use log::{debug, warn};
    use std::collections::HashSet;

    const SKIP_PREFIXES: &[&str] = &["/System/Volumes/", "/private/var/folders", "/dev", "/proc"];

    const VIRTUAL_FS: &[&str] = &[
        "devfs",
        "autofs",
        "synthfs",
        "nullfs",
        "macfuse",
        "nfs",
        "tmpfs",
        "apfs_snapshot",
    ];

    pub fn get_current_stats() -> Vec<PartitionInfo> {
        // Parse /etc/fstab isn't reliable on macOS; use mount output instead
        // TODO: consider falling back to statvfs on / or parsing diskutil info if mount fails
        let output = match std::process::Command::new("mount").output() {
            Ok(o) => o,
            Err(e) => {
                warn!("mount failed: {}", e);
                return vec![];
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut seen = HashSet::new();
        let mut disks = vec![];

        for line in stdout.lines() {
            // Format: "/dev/diskXsY on /mount/point (fstype, ...)"
            let mut parts = line.splitn(4, ' ');
            let device = parts.next().unwrap_or("");
            let _on = parts.next();
            let mount = parts.next().unwrap_or("");
            let meta = parts.next().unwrap_or("");

            // Skip virtual filesystems
            let fstype = meta
                .trim_start_matches('(')
                .split(',')
                .next()
                .unwrap_or("")
                .trim();
            if VIRTUAL_FS.iter().any(|v| fstype.eq_ignore_ascii_case(v)) {
                continue;
            }

            // Skip APFS system volumes and other noise
            if SKIP_PREFIXES.iter().any(|p| mount.starts_with(p)) {
                continue;
            }

            // Deduplicate by device path (handles APFS volume groups)
            if !seen.insert(device.to_string()) {
                continue;
            }

            if let Some(info) = super::statvfs_info(mount, device, device, fstype) {
                if info.total_bytes == 0 {
                    continue;
                }
                debug!(
                    "macOS disk: {} at {} — {:.1}GB total",
                    device,
                    mount,
                    info.total_bytes as f64 / 1_073_741_824.0
                );
                disks.push(info);
            }
        }

        disks.extend(collect_zpools());
        disks.sort_by(|a, b| a.name.cmp(&b.name));
        disks
    }
}

// ── Shared helpers ─────────────────────────────────────────────────────────────

/// Build a PartitionInfo from a mount point using statvfs.
/// `label` is used as the display name (device path or pool name).
fn statvfs_info(mount_point: &str, label: &str, device: &str, fs_type: &str) -> Option<PartitionInfo> {
    let stat = statvfs(mount_point).ok()?;

    let block_size = stat.fragment_size() as u64; // f_frsize -> the real unit
    let total_blocks = stat.blocks() as u64;
    let free_blocks = stat.blocks_free() as u64;
    let avail_blocks = stat.blocks_available() as u64; // unprivileged free (= df)

    let total_bytes = total_blocks * block_size;
    let available_bytes = avail_blocks * block_size;
    let used_bytes = total_bytes.saturating_sub(free_blocks * block_size);

    // Strip /dev/ prefix and partition suffix for a clean name
    let name = label.rsplit('/').next().unwrap_or(label).to_string();

    Some(PartitionInfo {
        name,
        device: device.to_string(),
        mount_point: mount_point.to_string(),
        fs_type: fs_type.to_string(),
        total_bytes,
        used_bytes,
        available_bytes,
        used_blocks: total_blocks.saturating_sub(free_blocks),
        available_blocks: avail_blocks,
        block_size,
        collected_at: Utc::now(),
    })
}

/// Query ZFS pools via `zpool list` works on both Linux and macOS.
fn collect_zpools() -> Vec<PartitionInfo> {
    let output = match Command::new("zpool")
        .args(["list", "-H", "-p", "-o", "name,size,alloc,free"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    let stdout = match std::str::from_utf8(&output.stdout) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    stdout
        .lines()
        .filter_map(|line| {
            let mut p = line.split('\t');
            let name = p.next()?.trim().to_string();
            let total_bytes: u64 = p.next()?.trim().parse().ok()?;
            let used_bytes: u64 = p.next()?.trim().parse().ok()?;
            let free_bytes: u64 = p.next()?.trim().parse().ok()?;

            // ZFS uses 512-byte ashift sectors by default report in those units
            let block_size: u64 = 512;

            Some(PartitionInfo {
                device: name.clone(),
                mount_point: format!("/{}", name),
                fs_type: "zfs".to_string(),
                name,
                total_bytes,
                used_bytes,
                available_bytes: free_bytes,
                used_blocks: used_bytes / block_size,
                available_blocks: free_bytes / block_size,
                block_size,
                collected_at: Utc::now(),
            })
        })
        .collect()
}

// ── Public entry point ─────────────────────────────────────────────────────────

impl PartitionInfo {
    #[cfg(target_os = "linux")]
    pub fn get_current_stats() -> Vec<PartitionInfo> {
        linux::get_current_stats()
    }

    #[cfg(target_os = "macos")]
    pub fn get_current_stats() -> Vec<PartitionInfo> {
        macos::get_current_stats()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn used_does_not_exceed_total() {
        for d in PartitionInfo::get_current_stats() {
            assert!(d.total_bytes > 0, "{}: total must be > 0", d.name);
            assert!(
                d.used_bytes <= d.total_bytes,
                "{}: used {}B > total {}B",
                d.name,
                d.used_bytes,
                d.total_bytes
            );
        }
    }

    #[test]
    fn blocks_consistent_with_bytes() {
        for d in PartitionInfo::get_current_stats() {
            if d.block_size == 0 {
                continue;
            }
            assert_eq!(
                d.used_blocks,
                d.used_bytes / d.block_size,
                "{}: used_blocks inconsistent",
                d.name
            );
            assert_eq!(
                d.available_blocks,
                d.available_bytes / d.block_size,
                "{}: available_blocks inconsistent",
                d.name
            );
        }
    }

    #[test]
    fn print_all_disks() {
        let disks = PartitionInfo::get_current_stats();

        assert!(!disks.is_empty(), "No disks found — something is wrong");

        println!("\n{:-<80}", "");
        println!(
            "{:<30} {:>12} {:>12} {:>12} {:>14}",
            "Name", "Total", "Used", "Available", "Blocks Free"
        );
        println!("{:-<80}", "");

        for disk in &disks {
            println!(
                "{:<30} {:>10.1}GB {:>10.1}GB {:>10.1}GB {:>14}",
                disk.name,
                disk.total_bytes as f64 / 1_073_741_824.0,
                disk.used_bytes as f64 / 1_073_741_824.0,
                disk.available_bytes as f64 / 1_073_741_824.0,
                disk.available_blocks,
            );
        }

        println!("{:-<80}", "");
        println!("Total disks found: {}", disks.len());

        println!("{:#?}", disks);
    }
}
