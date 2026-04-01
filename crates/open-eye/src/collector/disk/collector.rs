use nix::sys::statvfs::statvfs;
use std::collections::HashSet;
use std::process::Command;

#[cfg(not(target_os = "linux"))]
use sysinfo::Disks;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub used_blocks: u64,
    pub available_blocks: u64,
    pub block_size: u64,
}

// ── Linux ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux {
    use super::{collect_zpools, DiskInfo};
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
        fstype: Option<String>,
        #[serde(default)]
        mountpoint: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(rename = "type", default)]
        r#type: Option<String>,
        #[serde(default)]
        children: Option<Vec<BlockDevice>>,
    }

    fn is_physical_disk(dev: &BlockDevice) -> bool {
        // this is needed because the output of lsblk has a type field
        // and i rust it is a keyword and with the r#type i really need the field and not
        // the keywoard
        dev.r#type.as_deref() == Some("disk")
    }

    fn is_zfs_member(dev: &BlockDevice) -> bool {
        if dev.fstype.as_deref() == Some("zfs_member") {
            return true;
        }
        dev.children
            .as_ref()
            .map_or(false, |kids| kids.iter().any(is_zfs_member))
    }

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
        let output = Command::new("lsblk")
            .args([
                "-b",
                "-J",
                "-o",
                "NAME,SIZE,FSUSED,FSAVAIL,FSTYPE,MOUNTPOINT,MODEL,TYPE",
            ])
            .output();

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                warn!("lsblk failed: {}", e);
                return vec![];
            }
        };

        let json = match std::str::from_utf8(&output.stdout) {
            Ok(s) => s,
            Err(e) => {
                warn!("lsblk bad UTF-8: {}", e);
                return vec![];
            }
        };

        let parsed: LsblkOutput = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                warn!("lsblk JSON parse failed: {}", e);
                return vec![];
            }
        };

        let mut disks: Vec<DiskInfo> = parsed
            .blockdevices
            .iter()
            .filter(|dev| {
                if !is_physical_disk(dev) {
                    return false;
                }
                if dev.size.unwrap_or(0) == 0 {
                    return false;
                }
                if is_zfs_member(dev) {
                    debug!("skipping /dev/{}: ZFS member", dev.name);
                    return false;
                }
                true
            })
            .map(|dev| {
                let total_bytes = dev.size.unwrap_or(0);
                let used_bytes = sum_used(dev);
                let available_bytes = total_bytes.saturating_sub(used_bytes);
                // lsblk reports sizes in bytes; use 512-byte sectors as logical block unit
                let block_size: u64 = 512;
                let name = dev
                    .model
                    .as_deref()
                    .map(|m| m.trim())
                    .filter(|m| !m.is_empty())
                    .unwrap_or(&dev.name)
                    .to_string();
                debug!(
                    "accepted /dev/{} as \"{}\": {:.1}GB total",
                    dev.name,
                    name,
                    total_bytes as f64 / 1_073_741_824.0
                );
                DiskInfo {
                    name,
                    total_bytes,
                    used_bytes,
                    available_bytes,
                    used_blocks: used_bytes / block_size,
                    available_blocks: available_bytes / block_size,
                    block_size,
                }
            })
            .collect();

        disks.extend(collect_zpools());
        disks.sort_by(|a, b| a.name.cmp(&b.name));
        disks
    }
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use super::{collect_zpools, statvfs_info, DiskInfo};
    use log::debug;
    use std::collections::HashSet;
    use std::fs;

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

    pub fn collect() -> Vec<DiskInfo> {
        // Parse /etc/fstab isn't reliable on macOS; use mount output instead
        let output = std::process::Command::new("mount")
            .output()
            .unwrap_or_else(|_| panic!("failed to run mount"));

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

            if let Some(info) = super::statvfs_info(mount, device) {
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

/// Build a DiskInfo from a mount point using statvfs.
/// `label` is used as the display name (device path or pool name).
fn statvfs_info(mount_point: &str, label: &str) -> Option<DiskInfo> {
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

    Some(DiskInfo {
        name,
        total_bytes,
        used_bytes,
        available_bytes,
        used_blocks: total_blocks.saturating_sub(free_blocks),
        available_blocks: avail_blocks,
        block_size,
    })
}

/// Query ZFS pools via `zpool list` works on both Linux and macOS.
fn collect_zpools() -> Vec<DiskInfo> {
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

            Some(DiskInfo {
                name,
                total_bytes,
                used_bytes,
                available_bytes: free_bytes,
                used_blocks: used_bytes / block_size,
                available_blocks: free_bytes / block_size,
                block_size,
            })
        })
        .collect()
}

// ── Public entry point ─────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
pub fn collect() -> Vec<DiskInfo> {
    linux::collect()
}

#[cfg(target_os = "macos")]
pub fn collect() -> Vec<DiskInfo> {
    macos::collect()
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn used_does_not_exceed_total() {
        for d in collect() {
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
        for d in collect() {
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
        let disks = collect();

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
    }
}
