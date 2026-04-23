import glob
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


def p(label, value):
    if value is not None:
        print(f"  {label}: {value}")


def try_get(fn, *args, fmt=None):
    try:
        v = fn(*args)
        return fmt(v) if fmt else v
    except Exception:
        return None


# ── NVIDIA ────────────────────────────────────────────────────────────────────
try:
    pynvml.nvmlInit()

    driver = pynvml.nvmlSystemGetDriverVersion()
    nvml_ver = pynvml.nvmlSystemGetNVMLVersion()
    cuda_ver = pynvml.nvmlSystemGetCudaDriverVersion()
    print(f"NVIDIA driver {driver}  CUDA {cuda_ver // 1000}.{cuda_ver % 1000 // 10}  NVML {nvml_ver}\n")

    count = pynvml.nvmlDeviceGetCount()
    for i in range(count):
        h = pynvml.nvmlDeviceGetHandleByIndex(i)

        # ── Identity
        print(f"[NVIDIA] {pynvml.nvmlDeviceGetName(h)}")
        brand_names = {
            pynvml.NVML_BRAND_GEFORCE: "GeForce", pynvml.NVML_BRAND_GEFORCE_RTX: "GeForce RTX",
            pynvml.NVML_BRAND_QUADRO: "Quadro", pynvml.NVML_BRAND_TESLA: "Tesla",
            pynvml.NVML_BRAND_NVIDIA_RTX: "NVIDIA RTX", pynvml.NVML_BRAND_TITAN_RTX: "TITAN RTX",
            pynvml.NVML_BRAND_TITAN: "TITAN",
        }
        p("Brand",    try_get(pynvml.nvmlDeviceGetBrand, h, fmt=lambda v: brand_names.get(v, str(v))))
        p("UUID",     try_get(pynvml.nvmlDeviceGetUUID, h))
        p("Serial",   try_get(pynvml.nvmlDeviceGetSerial, h))
        p("VBIOS",    try_get(pynvml.nvmlDeviceGetVbiosVersion, h))
        p("Board P/N",try_get(pynvml.nvmlDeviceGetBoardPartNumber, h))
        p("Arch",     try_get(pynvml.nvmlDeviceGetArchitecture, h))
        p("Multi-GPU",try_get(pynvml.nvmlDeviceGetMultiGpuBoard, h, fmt=lambda v: "yes" if v else "no"))

        cuda_maj, cuda_min = try_get(pynvml.nvmlDeviceGetCudaComputeCapability, h) or (None, None)
        if cuda_maj:
            p("CUDA cap", f"{cuda_maj}.{cuda_min}")

        # ── PCI
        pci = try_get(pynvml.nvmlDeviceGetPciInfo, h)
        if pci:
            p("PCI bus", pci.busId)
        p("PCIe link",   try_get(lambda: f"Gen{pynvml.nvmlDeviceGetCurrPcieLinkGeneration(h)} x{pynvml.nvmlDeviceGetCurrPcieLinkWidth(h)}"))
        p("PCIe max",    try_get(lambda: f"Gen{pynvml.nvmlDeviceGetMaxPcieLinkGeneration(h)} x{pynvml.nvmlDeviceGetMaxPcieLinkWidth(h)}"))

        # ── Memory
        mem = try_get(pynvml.nvmlDeviceGetMemoryInfo, h)
        if mem:
            p("VRAM", f"{mem.used // 1024**2} / {mem.total // 1024**2} MB  (free {mem.free // 1024**2} MB)")
        bar1 = try_get(pynvml.nvmlDeviceGetBAR1MemoryInfo, h)
        if bar1:
            p("BAR1", f"{bar1.bar1Used // 1024**2} / {bar1.bar1Total // 1024**2} MB")
        p("Mem bus width", try_get(pynvml.nvmlDeviceGetMemoryBusWidth, h, fmt=lambda v: f"{v}-bit"))

        # ── Clocks
        for clock_type, label in [
            (pynvml.NVML_CLOCK_GRAPHICS, "GPU clock"),
            (pynvml.NVML_CLOCK_MEM,      "Mem clock"),
            (pynvml.NVML_CLOCK_SM,       "SM clock"),
            (pynvml.NVML_CLOCK_VIDEO,    "Video clock"),
        ]:
            cur  = try_get(pynvml.nvmlDeviceGetClockInfo, h, clock_type)
            max_ = try_get(pynvml.nvmlDeviceGetMaxClockInfo, h, clock_type)
            if cur is not None:
                p(label, f"{cur} / {max_} MHz" if max_ else f"{cur} MHz")

        # ── Temperature & thermals
        temp = try_get(pynvml.nvmlDeviceGetTemperature, h, pynvml.NVML_TEMPERATURE_GPU)
        t_slow = try_get(pynvml.nvmlDeviceGetTemperatureThreshold, h, pynvml.NVML_TEMPERATURE_THRESHOLD_SLOWDOWN)
        t_shut = try_get(pynvml.nvmlDeviceGetTemperatureThreshold, h, pynvml.NVML_TEMPERATURE_THRESHOLD_SHUTDOWN)
        if temp is not None:
            parts = [f"{temp}°C"]
            if t_slow: parts.append(f"slowdown {t_slow}°C")
            if t_shut: parts.append(f"shutdown {t_shut}°C")
            p("Temp", "  ".join(parts))

        # ── Power
        power = try_get(pynvml.nvmlDeviceGetPowerUsage, h)
        plimit = try_get(pynvml.nvmlDeviceGetPowerManagementLimit, h)
        plimit_enf = try_get(pynvml.nvmlDeviceGetEnforcedPowerLimit, h)
        if power is not None:
            p("Power", f"{power / 1000:.1f} W  (limit {plimit / 1000:.0f} W  enforced {plimit_enf / 1000:.0f} W)" if plimit else f"{power / 1000:.1f} W")
        p("Energy total", try_get(pynvml.nvmlDeviceGetTotalEnergyConsumption, h, fmt=lambda v: f"{v / 1000:.1f} kJ"))

        # ── Fans
        num_fans = try_get(pynvml.nvmlDeviceGetNumFans, h) or 0
        if num_fans:
            for fi in range(num_fans):
                speed = try_get(pynvml.nvmlDeviceGetFanSpeed, h, fi)
                p(f"Fan {fi}", f"{speed}%" if speed is not None else "N/A")
        else:
            p("Fan", try_get(pynvml.nvmlDeviceGetFanSpeed, h, fmt=lambda v: f"{v}%"))

        # ── Utilization
        util = try_get(pynvml.nvmlDeviceGetUtilizationRates, h)
        if util:
            p("GPU util",    f"{util.gpu}%")
            p("Mem util",    f"{util.memory}%")
        enc = try_get(pynvml.nvmlDeviceGetEncoderUtilization, h)
        dec = try_get(pynvml.nvmlDeviceGetDecoderUtilization, h)
        if enc: p("Encoder util", f"{enc[0]}%")
        if dec: p("Decoder util", f"{dec[0]}%")

        # ── State & modes
        p("Perf state",   try_get(pynvml.nvmlDeviceGetPerformanceState, h, fmt=lambda v: f"P{v}"))
        p("Compute mode", try_get(pynvml.nvmlDeviceGetComputeMode, h))
        p("Persistence",  try_get(pynvml.nvmlDeviceGetPersistenceMode, h, fmt=lambda v: "on" if v else "off"))
        p("Display",      try_get(pynvml.nvmlDeviceGetDisplayActive, h, fmt=lambda v: "active" if v else "inactive"))
        p("ECC",          try_get(pynvml.nvmlDeviceGetCurrentEccMode, h, fmt=lambda v: "on" if v else "off"))

        print()

    pynvml.nvmlShutdown()
except pynvml.NVMLError:
    print("No NVIDIA GPU found\n")


# ── AMD (discrete + integrated APUs) ─────────────────────────────────────────
try:
    count = pyamdgpuinfo.detect_gpus()
    if count == 0:
        print("No AMD GPU found\n")
    for i in range(count):
        gpu = pyamdgpuinfo.get_gpu(i)
        print(f"[AMD] {gpu.name}")

        # Memory
        try:
            used = gpu.query_vram_usage()
            total = gpu.memory_info["vram_size"]
            p("VRAM", f"{used // 1024**2} / {total // 1024**2} MB")
        except Exception:
            pass
        try:
            used = gpu.query_gtt_usage()
            total = gpu.memory_info["gtt_size"]
            p("GTT", f"{used // 1024**2} / {total // 1024**2} MB")
        except Exception:
            pass

        # Clocks
        try:
            p("GPU clock", f"{gpu.query_sclk() // 1_000_000} MHz")
        except Exception:
            pass
        try:
            p("Mem clock", f"{gpu.query_mclk() // 1_000_000} MHz")
        except Exception:
            pass
        try:
            maxc = gpu.query_max_clocks()
            p("GPU clock max", f"{maxc['sclk'] // 1_000_000} MHz")
            p("Mem clock max", f"{maxc['mclk'] // 1_000_000} MHz")
        except Exception:
            pass

        # Thermals & power
        try:
            p("Temp",  f"{gpu.query_temperature():.0f}°C")
        except Exception:
            pass
        try:
            p("Power", f"{gpu.query_power():.1f} W")
        except Exception:
            pass

        # Voltages
        try:
            p("NB voltage",  f"{gpu.query_northbridge_voltage():.3f} V")
        except Exception:
            pass
        try:
            p("GPU voltage", f"{gpu.query_graphics_voltage():.3f} V")
        except Exception:
            pass

        # Utilization
        try:
            p("GPU util", f"{gpu.query_load() * 100:.1f}%")
        except Exception:
            pass

        print()
except Exception as e:
    print(f"No AMD GPU found: {e}\n")
