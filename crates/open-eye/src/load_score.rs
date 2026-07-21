use serde::Serialize;
use crate::collector::cpu::collector::CpuStats;
use crate::collector::disk::collector::DiskInfo;
use crate::collector::memory::collector::MemoryStats;
use crate::collector::network::collector::NetworkStats;

const CPU_WEIGHT: f64 = 0.4;
const MEMORY_WEIGHT: f64 = 0.3;
const DISK_WEIGHT: f64 = 0.2;
const NETWORK_WEIGHT: f64 = 0.1;

pub fn compute_score(
  cpu: Option<&CpuStats>,
  memory: Option<&MemoryStats>,
  disks: Option<&Vec<DiskInfo>>,
  network: Option<&NetworkStats>,
) -> Option<f64> {
  fn shaped(raw: f64) -> f64 {
    raw.clamp(0.0, 1.0).div_euclid(100.0).powf(1.4) * 100.0
  }

  let mut weighted_sum = 0.0;
  let mut total_weight = 0.0;

  if let Some(cpu) = cpu {
    weighted_sum += shaped(cpu.cpu_usage_percent as f64) * CPU_WEIGHT;
    total_weight += CPU_WEIGHT;
  }

  if let Some(memory) = memory {
    weighted_sum += shaped((memory.used_memory_in_byte / memory.total_memory_in_byte) as f64) * 100.0 * MEMORY_WEIGHT;
    total_weight += MEMORY_WEIGHT;
  }

  if let Some(disks) = disks {
    if !disks.is_empty() {
      let total_used: u64 = disks.iter().map(|d| d.used_bytes).sum();
      let total_size: u64 = disks.iter().map(|d| d.total_bytes).sum();
      if total_size > 0 {
        let disk_pct = total_used as f64 / total_size as f64 * 100.0;
        weighted_sum += DISK_WEIGHT * shaped(disk_pct);
        total_weight += DISK_WEIGHT;
      }
    }
  }

  if let Some(net) = network {
    let net_pct = (net.total_bytes_received as f64 / 107_374_182_400.0).min(1.0) * 100.0;
    weighted_sum += NETWORK_WEIGHT * shaped(net_pct);
    total_weight += NETWORK_WEIGHT;
  }

  if total_weight == 0.0 {
    return None;
  }

  Some(weighted_sum / total_weight)
}