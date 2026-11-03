use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::collector::DataCreationTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub usage_percent: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub temperature_celsius: Option<f32>,
    pub collected_at: chrono::DateTime<Utc>,
}

impl DataCreationTime for GpuInfo {
    fn get_data_creation_time(&self) -> i64 {
        self.collected_at.timestamp()
    }
}

pub struct GpuStats;

impl GpuStats {
    pub fn get_current_stats() -> Vec<GpuInfo> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        let mut gpus = nvidia::collect();
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let mut gpus: Vec<GpuInfo> = Vec::new();

        #[cfg(target_os = "linux")]
        gpus.extend(linux::collect(!gpus.is_empty()));
        #[cfg(target_os = "windows")]
        gpus.extend(windows::collect(!gpus.is_empty()));
        #[cfg(target_os = "macos")]
        gpus.extend(macos::collect());

        gpus
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
mod nvidia {
    use super::GpuInfo;
    use chrono::Utc;
    use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
    use nvml_wrapper::Nvml;
    use std::sync::OnceLock;

    static NVML: OnceLock<Option<Nvml>> = OnceLock::new();

    fn nvml() -> Option<&'static Nvml> {
        NVML.get_or_init(|| match Nvml::init() {
            Ok(nvml) => Some(nvml),
            Err(e) => {
                log::debug!("nvml unavailable: {e}");
                None
            }
        })
        .as_ref()
    }

    pub fn collect() -> Vec<GpuInfo> {
        let Some(nvml) = nvml() else {
            return Vec::new();
        };
        let count = match nvml.device_count() {
            Ok(count) => count,
            Err(e) => {
                log::warn!("nvml device count failed: {e}");
                return Vec::new();
            }
        };

        (0..count)
            .filter_map(|index| match nvml.device_by_index(index) {
                Ok(device) => {
                    let memory = device.memory_info().ok();
                    Some(GpuInfo {
                        name: device.name().unwrap_or_else(|_| format!("NVIDIA GPU {index}")),
                        usage_percent: device.utilization_rates().ok().map(|u| u.gpu as f32),
                        memory_used_bytes: memory.as_ref().map(|m| m.used),
                        memory_total_bytes: memory.as_ref().map(|m| m.total),
                        temperature_celsius: device.temperature(TemperatureSensor::Gpu).ok().map(|t| t as f32),
                        collected_at: Utc::now(),
                    })
                }
                Err(e) => {
                    log::warn!("nvml device {index} failed: {e}");
                    None
                }
            })
            .collect()
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::GpuInfo;
    use chrono::Utc;
    use std::fs;
    use std::path::Path;

    fn read_trimmed(path: &Path) -> Option<String> {
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    fn read_u64(path: &Path) -> Option<u64> {
        read_trimmed(path)?.parse().ok()
    }

    fn hwmon_temperature(device: &Path) -> Option<f32> {
        fs::read_dir(device.join("hwmon"))
            .ok()?
            .flatten()
            .find_map(|entry| read_u64(&entry.path().join("temp1_input")))
            .map(|millidegrees| millidegrees as f32 / 1000.0)
    }

    fn is_card(name: &str) -> bool {
        name.strip_prefix("card")
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
    }

    pub fn collect(nvidia_reported: bool) -> Vec<GpuInfo> {
        let Ok(entries) = fs::read_dir("/sys/class/drm") else {
            return Vec::new();
        };

        let mut cards: Vec<_> = entries
            .flatten()
            .filter(|entry| is_card(&entry.file_name().to_string_lossy()))
            .map(|entry| entry.path().join("device"))
            .collect();
        cards.sort();

        cards
            .into_iter()
            .filter_map(|device| {
                let vendor = read_trimmed(&device.join("vendor"))?;
                let vendor_name = match vendor.as_str() {
                    "0x10de" if nvidia_reported => return None,
                    "0x10de" => "NVIDIA",
                    "0x1002" => "AMD",
                    "0x8086" => "Intel",
                    _ => return None,
                };
                let slot = fs::canonicalize(&device)
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_default();
                let name = read_trimmed(&device.join("product_name"))
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| format!("{vendor_name} GPU {slot}"));

                Some(GpuInfo {
                    name,
                    usage_percent: read_u64(&device.join("gpu_busy_percent")).map(|p| p as f32),
                    memory_used_bytes: read_u64(&device.join("mem_info_vram_used")),
                    memory_total_bytes: read_u64(&device.join("mem_info_vram_total")),
                    temperature_celsius: hwmon_temperature(&device),
                    collected_at: Utc::now(),
                })
            })
            .collect()
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::GpuInfo;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::time::Duration;
    use windows::core::{PCWSTR, HSTRING};
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_FMT_LARGE, PDH_HCOUNTER,
        PDH_HQUERY, PDH_MORE_DATA,
    };

    struct Adapter {
        luid: String,
        name: String,
        vendor_id: u32,
        dedicated_memory: u64,
    }

    fn adapters() -> Vec<Adapter> {
        let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
            Ok(factory) => factory,
            Err(e) => {
                log::warn!("dxgi factory failed: {e}");
                return Vec::new();
            }
        };

        let mut adapters = Vec::new();
        let mut index = 0;
        while let Ok(adapter) = unsafe { factory.EnumAdapters1(index) } {
            index += 1;
            let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
                continue;
            };
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            let name_len = desc.Description.iter().position(|&c| c == 0).unwrap_or(desc.Description.len());
            adapters.push(Adapter {
                luid: format!(
                    "luid_0x{:08x}_0x{:08x}",
                    desc.AdapterLuid.HighPart as u32, desc.AdapterLuid.LowPart
                ),
                name: String::from_utf16_lossy(&desc.Description[..name_len]),
                vendor_id: desc.VendorId,
                dedicated_memory: desc.DedicatedVideoMemory as u64,
            });
        }
        adapters
    }

    fn counter_items(counter: PDH_HCOUNTER, format: u32) -> Vec<(String, PDH_FMT_COUNTERVALUE_ITEM_W)> {
        let mut size = 0u32;
        let mut count = 0u32;
        let status = unsafe { PdhGetFormattedCounterArrayW(counter, format, &mut size, &mut count, None) };
        if status != PDH_MORE_DATA || size == 0 {
            return Vec::new();
        }

        let item_size = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let mut buffer = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); (size as usize).div_ceil(item_size)];
        let status = unsafe {
            PdhGetFormattedCounterArrayW(counter, format, &mut size, &mut count, Some(buffer.as_mut_ptr()))
        };
        if status != 0 {
            return Vec::new();
        }

        buffer
            .into_iter()
            .take(count as usize)
            .filter_map(|item| {
                let name = unsafe { item.szName.to_string() }.ok()?;
                Some((name, item))
            })
            .collect()
    }

    struct Samples {
        usage: HashMap<String, f32>,
        memory_used: HashMap<String, u64>,
    }

    fn sample() -> Option<Samples> {
        let mut query = PDH_HQUERY::default();
        if unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) } != 0 {
            return None;
        }

        let mut engine = PDH_HCOUNTER::default();
        let mut memory = PDH_HCOUNTER::default();
        let engine_path = HSTRING::from(r"\GPU Engine(*)\Utilization Percentage");
        let memory_path = HSTRING::from(r"\GPU Adapter Memory(*)\Dedicated Usage");
        let added = unsafe {
            PdhAddEnglishCounterW(query, &engine_path, 0, &mut engine) == 0
                && PdhAddEnglishCounterW(query, &memory_path, 0, &mut memory) == 0
        };
        if !added {
            unsafe { PdhCloseQuery(query) };
            return None;
        }

        unsafe { PdhCollectQueryData(query) };
        std::thread::sleep(Duration::from_millis(500));
        unsafe { PdhCollectQueryData(query) };

        let mut per_engine_type: HashMap<(String, String), f32> = HashMap::new();
        for (name, item) in counter_items(engine, PDH_FMT_DOUBLE.0) {
            let (Some(luid), Some(engine_type)) = (
                name.find("luid_").map(|i| name[i..].chars().take(26).collect::<String>().to_ascii_lowercase()),
                name.find("engtype_").map(|i| name[i + 8..].to_string()),
            ) else {
                continue;
            };
            let value = unsafe { item.FmtValue.Anonymous.doubleValue } as f32;
            *per_engine_type.entry((luid, engine_type)).or_default() += value;
        }

        let mut usage: HashMap<String, f32> = HashMap::new();
        for ((luid, _), value) in per_engine_type {
            let entry = usage.entry(luid).or_default();
            *entry = entry.max(value.min(100.0));
        }

        let mut memory_used: HashMap<String, u64> = HashMap::new();
        for (name, item) in counter_items(memory, PDH_FMT_LARGE.0) {
            let Some(luid) = name.find("luid_").map(|i| name[i..].chars().take(26).collect::<String>().to_ascii_lowercase()) else {
                continue;
            };
            let value = unsafe { item.FmtValue.Anonymous.largeValue }.max(0) as u64;
            *memory_used.entry(luid).or_default() += value;
        }

        unsafe { PdhCloseQuery(query) };
        Some(Samples { usage, memory_used })
    }

    pub fn collect(nvidia_reported: bool) -> Vec<GpuInfo> {
        let adapters: Vec<Adapter> = adapters()
            .into_iter()
            .filter(|a| !(nvidia_reported && a.vendor_id == 0x10de))
            .collect();
        if adapters.is_empty() {
            return Vec::new();
        }

        let samples = sample();
        if samples.is_none() {
            log::warn!("gpu performance counters unavailable");
        }

        adapters
            .into_iter()
            .map(|adapter| GpuInfo {
                usage_percent: samples.as_ref().and_then(|s| s.usage.get(&adapter.luid).copied()),
                memory_used_bytes: samples.as_ref().and_then(|s| s.memory_used.get(&adapter.luid).copied()),
                memory_total_bytes: (adapter.dedicated_memory > 0).then_some(adapter.dedicated_memory),
                temperature_celsius: None,
                name: adapter.name,
                collected_at: Utc::now(),
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::GpuInfo;
    use chrono::Utc;
    use plist::{Dictionary, Value};
    use std::process::Command;

    fn number(dict: &Dictionary, key: &str) -> Option<u64> {
        match dict.get(key)? {
            Value::Integer(i) => i.as_unsigned().or_else(|| i.as_signed().map(|v| v.max(0) as u64)),
            Value::Real(r) => Some(r.max(0.0) as u64),
            _ => None,
        }
    }

    pub fn collect() -> Vec<GpuInfo> {
        let output = match Command::new("ioreg")
            .args(["-r", "-d", "1", "-w0", "-c", "IOAccelerator", "-a"])
            .output()
        {
            Ok(output) if output.status.success() => output.stdout,
            Ok(output) => {
                log::warn!("ioreg exited with {}", output.status);
                return Vec::new();
            }
            Err(e) => {
                log::warn!("ioreg failed: {e}");
                return Vec::new();
            }
        };

        let accelerators = match Value::from_reader(std::io::Cursor::new(output)) {
            Ok(Value::Array(items)) => items,
            Ok(_) => return Vec::new(),
            Err(e) => {
                log::warn!("ioreg plist parse failed: {e}");
                return Vec::new();
            }
        };

        accelerators
            .iter()
            .filter_map(Value::as_dictionary)
            .map(|accelerator| {
                let stats = accelerator
                    .get("PerformanceStatistics")
                    .and_then(Value::as_dictionary);
                let name = accelerator
                    .get("model")
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.clone()),
                        Value::Data(d) => Some(String::from_utf8_lossy(d).trim_end_matches('\0').to_string()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "Apple GPU".to_string());

                GpuInfo {
                    name,
                    usage_percent: stats
                        .and_then(|s| number(s, "Device Utilization %"))
                        .map(|p| p.min(100) as f32),
                    memory_used_bytes: stats.and_then(|s| {
                        number(s, "In use system memory").or_else(|| number(s, "vramUsedBytes"))
                    }),
                    memory_total_bytes: number(accelerator, "VRAM,totalMB").map(|mb| mb * 1024 * 1024),
                    temperature_celsius: None,
                    collected_at: Utc::now(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::collector::gpu::collector::GpuStats;

    #[test]
    fn get_current_gpu_stats_test() {
        let gpus = GpuStats::get_current_stats();

        for gpu in &gpus {
            println!("{gpu:?}");
            assert!(!gpu.name.is_empty(), "gpu name should not be empty");
            if let Some(usage) = gpu.usage_percent {
                assert!((0.0..=100.0).contains(&usage), "usage ({usage}) should be within 0..=100");
            }
            if let (Some(used), Some(total)) = (gpu.memory_used_bytes, gpu.memory_total_bytes) {
                assert!(used <= total, "used memory ({used}) should be <= total memory ({total})");
            }
        }
    }
}
