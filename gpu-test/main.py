import glob
import json
import os
import subprocess

import pynvml
import pyamdgpuinfo


def sysfs(path):
    try:
        with open(path) as f:
            return f.read().strip()
    except OSError:
        return None


def try_get(fn, *args, fmt=None):
    try:
        v = fn(*args)
        return fmt(v) if fmt else v
    except Exception:
        return None


def get_nvidia_info() -> dict | None:
    try:
        pynvml.nvmlInit()
    except pynvml.NVMLError:
        return None

    try:
        cuda_ver = pynvml.nvmlSystemGetCudaDriverVersion()
        result = {
            "driver":      pynvml.nvmlSystemGetDriverVersion(),
            "cuda":        f"{cuda_ver // 1000}.{cuda_ver % 1000 // 10}",
            "nvml":        pynvml.nvmlSystemGetNVMLVersion(),
            "gpus":        [],
        }

        brand_names = {
            pynvml.NVML_BRAND_GEFORCE:     "GeForce",
            pynvml.NVML_BRAND_GEFORCE_RTX: "GeForce RTX",
            pynvml.NVML_BRAND_QUADRO:      "Quadro",
            pynvml.NVML_BRAND_TESLA:       "Tesla",
            pynvml.NVML_BRAND_NVIDIA_RTX:  "NVIDIA RTX",
            pynvml.NVML_BRAND_TITAN_RTX:   "TITAN RTX",
            pynvml.NVML_BRAND_TITAN:       "TITAN",
        }

        for i in range(pynvml.nvmlDeviceGetCount()):
            h = pynvml.nvmlDeviceGetHandleByIndex(i)

            mem   = try_get(pynvml.nvmlDeviceGetMemoryInfo, h)
            bar1  = try_get(pynvml.nvmlDeviceGetBAR1MemoryInfo, h)
            util  = try_get(pynvml.nvmlDeviceGetUtilizationRates, h)
            enc   = try_get(pynvml.nvmlDeviceGetEncoderUtilization, h)
            dec   = try_get(pynvml.nvmlDeviceGetDecoderUtilization, h)
            pci   = try_get(pynvml.nvmlDeviceGetPciInfo, h)
            power = try_get(pynvml.nvmlDeviceGetPowerUsage, h)
            plim  = try_get(pynvml.nvmlDeviceGetPowerManagementLimit, h)
            plim_enf = try_get(pynvml.nvmlDeviceGetEnforcedPowerLimit, h)
            temp  = try_get(pynvml.nvmlDeviceGetTemperature, h, pynvml.NVML_TEMPERATURE_GPU)
            cuda_cap = try_get(pynvml.nvmlDeviceGetCudaComputeCapability, h)

            clocks = {}
            for clock_type, key in [
                (pynvml.NVML_CLOCK_GRAPHICS, "gpu"),
                (pynvml.NVML_CLOCK_MEM,      "mem"),
                (pynvml.NVML_CLOCK_SM,       "sm"),
                (pynvml.NVML_CLOCK_VIDEO,    "video"),
            ]:
                cur  = try_get(pynvml.nvmlDeviceGetClockInfo, h, clock_type)
                max_ = try_get(pynvml.nvmlDeviceGetMaxClockInfo, h, clock_type)
                if cur is not None:
                    clocks[f"{key}_mhz"]     = cur
                    clocks[f"{key}_max_mhz"] = max_

            num_fans = try_get(pynvml.nvmlDeviceGetNumFans, h) or 0
            fans = []
            if num_fans:
                for fi in range(num_fans):
                    fans.append(try_get(pynvml.nvmlDeviceGetFanSpeed, h, fi))
            else:
                speed = try_get(pynvml.nvmlDeviceGetFanSpeed, h)
                if speed is not None:
                    fans.append(speed)

            gpu = {
                "name":               pynvml.nvmlDeviceGetName(h),
                "brand":              try_get(pynvml.nvmlDeviceGetBrand, h, fmt=lambda v: brand_names.get(v, str(v))),
                "uuid":               try_get(pynvml.nvmlDeviceGetUUID, h),
                "serial":             try_get(pynvml.nvmlDeviceGetSerial, h),
                "vbios":              try_get(pynvml.nvmlDeviceGetVbiosVersion, h),
                "board_part_number":  try_get(pynvml.nvmlDeviceGetBoardPartNumber, h),
                "architecture":       try_get(pynvml.nvmlDeviceGetArchitecture, h),
                "multi_gpu":          try_get(pynvml.nvmlDeviceGetMultiGpuBoard, h, fmt=bool),
                "cuda_compute_cap":   f"{cuda_cap[0]}.{cuda_cap[1]}" if cuda_cap else None,
                "pci_bus_id":         pci.busId if pci else None,
                "pcie_link_gen":      try_get(pynvml.nvmlDeviceGetCurrPcieLinkGeneration, h),
                "pcie_link_width":    try_get(pynvml.nvmlDeviceGetCurrPcieLinkWidth, h),
                "pcie_max_gen":       try_get(pynvml.nvmlDeviceGetMaxPcieLinkGeneration, h),
                "pcie_max_width":     try_get(pynvml.nvmlDeviceGetMaxPcieLinkWidth, h),
                "mem_bus_width_bits": try_get(pynvml.nvmlDeviceGetMemoryBusWidth, h),
                "vram_used_mb":       mem.used  // 1024**2 if mem else None,
                "vram_total_mb":      mem.total // 1024**2 if mem else None,
                "vram_free_mb":       mem.free  // 1024**2 if mem else None,
                "bar1_used_mb":       bar1.bar1Used  // 1024**2 if bar1 else None,
                "bar1_total_mb":      bar1.bar1Total // 1024**2 if bar1 else None,
                "clocks":             clocks,
                "temp_celsius":       temp,
                "temp_slowdown_celsius": try_get(pynvml.nvmlDeviceGetTemperatureThreshold, h, pynvml.NVML_TEMPERATURE_THRESHOLD_SLOWDOWN),
                "temp_shutdown_celsius": try_get(pynvml.nvmlDeviceGetTemperatureThreshold, h, pynvml.NVML_TEMPERATURE_THRESHOLD_SHUTDOWN),
                "power_usage_watts":  power / 1000 if power is not None else None,
                "power_limit_watts":  plim  / 1000 if plim  is not None else None,
                "power_enforced_limit_watts": plim_enf / 1000 if plim_enf is not None else None,
                "energy_total_kj":    try_get(pynvml.nvmlDeviceGetTotalEnergyConsumption, h, fmt=lambda v: v / 1000),
                "fan_speed_percent":  fans,
                "gpu_util_percent":   util.gpu    if util else None,
                "mem_util_percent":   util.memory if util else None,
                "encoder_util_percent": enc[0] if enc else None,
                "decoder_util_percent": dec[0] if dec else None,
                "perf_state":         try_get(pynvml.nvmlDeviceGetPerformanceState, h, fmt=lambda v: f"P{v}"),
                "compute_mode":       try_get(pynvml.nvmlDeviceGetComputeMode, h),
                "persistence_mode":   try_get(pynvml.nvmlDeviceGetPersistenceMode, h, fmt=bool),
                "display_active":     try_get(pynvml.nvmlDeviceGetDisplayActive, h, fmt=bool),
                "ecc_enabled":        try_get(pynvml.nvmlDeviceGetCurrentEccMode, h, fmt=bool),
            }
            result["gpus"].append(gpu)

        return result
    finally:
        pynvml.nvmlShutdown()


def get_amd_info() -> list[dict] | None:
    try:
        count = pyamdgpuinfo.detect_gpus()
    except Exception:
        return None

    if count == 0:
        return None

    gpus = []
    for i in range(count):
        gpu = pyamdgpuinfo.get_gpu(i)

        max_clocks = try_get(gpu.query_max_clocks)

        gpus.append({
            "name":              gpu.name,
            "vram_used_mb":      try_get(gpu.query_vram_usage, fmt=lambda v: v // 1024**2),
            "vram_total_mb":     try_get(lambda: gpu.memory_info["vram_size"] // 1024**2),
            "gtt_used_mb":       try_get(gpu.query_gtt_usage, fmt=lambda v: v // 1024**2),
            "gtt_total_mb":      try_get(lambda: gpu.memory_info["gtt_size"] // 1024**2),
            "gpu_clock_mhz":     try_get(gpu.query_sclk, fmt=lambda v: v // 1_000_000),
            "mem_clock_mhz":     try_get(gpu.query_mclk, fmt=lambda v: v // 1_000_000),
            "gpu_clock_max_mhz": max_clocks["sclk_max"] // 1_000_000 if max_clocks else None,
            "mem_clock_max_mhz": max_clocks["mclk_max"] // 1_000_000 if max_clocks else None,
            "temp_celsius":      try_get(gpu.query_temperature),
            "power_usage_watts": try_get(gpu.query_power),
            "nb_voltage":        try_get(gpu.query_northbridge_voltage),
            "gpu_voltage":       try_get(gpu.query_graphics_voltage),
            "gpu_util_percent":  try_get(gpu.query_load, fmt=lambda v: v * 100),
        })

    return gpus


def get_intel_info() -> list[dict] | None:
    gpus = []

    for card in sorted(glob.glob("/sys/class/drm/card[0-9]*")):
        if sysfs(os.path.join(card, "device", "vendor")) != "0x8086":
            continue

        dev_id = sysfs(os.path.join(card, "device", "device")) or "unknown"
        try:
            result = subprocess.run(
                ["lspci", "-d", f"8086:{dev_id[2:]}"],
                capture_output=True, text=True
            )
            name = result.stdout.split(": ", 1)[-1].strip() if result.stdout else f"Intel GPU ({dev_id})"
        except Exception:
            name = f"Intel GPU ({dev_id})"

        def first_sysfs(*paths):
            for path in paths:
                val = sysfs(os.path.join(card, path))
                if val is not None:
                    return val
            return None

        temp = power = energy = None
        for hwmon in glob.glob(os.path.join(card, "device/hwmon/hwmon*")):
            raw = sysfs(os.path.join(hwmon, "temp1_input"))
            if raw:
                temp = int(raw) / 1000
            raw = sysfs(os.path.join(hwmon, "power1_input"))
            if raw:
                power = int(raw) / 1_000_000
            raw = sysfs(os.path.join(hwmon, "energy1_input"))
            if raw:
                energy = int(raw) / 1_000_000

        def mb(path):
            v = sysfs(os.path.join(card, "device", path))
            return int(v) // 1024**2 if v else None

        def mhz(val):
            return int(val) if val else None

        gpus.append({
            "name":               name,
            "pci_device_id":      dev_id,
            "subsystem_vendor":   sysfs(os.path.join(card, "device", "subsystem_vendor")),
            "subsystem_device":   sysfs(os.path.join(card, "device", "subsystem_device")),
            "cur_freq_mhz":       mhz(first_sysfs("gt_cur_freq_mhz",   "gt/gt0/rps_cur_freq_mhz")),
            "act_freq_mhz":       mhz(first_sysfs("gt_act_freq_mhz",   "gt/gt0/rps_act_freq_mhz")),
            "min_freq_mhz":       mhz(first_sysfs("gt_min_freq_mhz",   "gt/gt0/rps_min_freq_mhz")),
            "max_freq_mhz":       mhz(first_sysfs("gt_max_freq_mhz",   "gt/gt0/rps_max_freq_mhz")),
            "boost_freq_mhz":     mhz(first_sysfs("gt_boost_freq_mhz", "gt/gt0/rps_boost_freq_mhz")),
            "rp0_freq_mhz":       mhz(first_sysfs("gt_RP0_freq_mhz")),
            "rp1_freq_mhz":       mhz(first_sysfs("gt_RP1_freq_mhz")),
            "rpn_freq_mhz":       mhz(first_sysfs("gt_RPn_freq_mhz")),
            "vram_used_mb":       mb("mem_info_vram_used"),
            "vram_total_mb":      mb("mem_info_vram_total"),
            "gtt_used_mb":        mb("mem_info_gtt_used"),
            "gtt_total_mb":       mb("mem_info_gtt_total"),
            "vis_vram_used_mb":   mb("mem_info_vis_vram_used"),
            "vis_vram_total_mb":  mb("mem_info_vis_vram_total"),
            "temp_celsius":       temp,
            "power_watts":        power,
            "energy_joules":      energy,
            "runtime_status":     sysfs(os.path.join(card, "device/power/runtime_status")),
        })

    return gpus if gpus else None


if __name__ == "__main__":
    print(json.dumps({
        "nvidia": get_nvidia_info(),
        "amd":    get_amd_info(),
        "intel":  get_intel_info(),
    }, indent=2))
