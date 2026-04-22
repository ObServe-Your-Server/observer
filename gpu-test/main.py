import pynvml
import pyamdgpuinfo

# NVIDIA
try:
    pynvml.nvmlInit()
    count = pynvml.nvmlDeviceGetCount()
    for i in range(count):
        handle = pynvml.nvmlDeviceGetHandleByIndex(i)
        mem = pynvml.nvmlDeviceGetMemoryInfo(handle)
        util = pynvml.nvmlDeviceGetUtilizationRates(handle)
        print(f"[NVIDIA] {pynvml.nvmlDeviceGetName(handle)}")
        print(f"  VRAM:  {mem.used // 1024**2} / {mem.total // 1024**2} MB")
        print(f"  Temp:  {pynvml.nvmlDeviceGetTemperature(handle, pynvml.NVML_TEMPERATURE_GPU)}°C")
        print(f"  Power: {pynvml.nvmlDeviceGetPowerUsage(handle) / 1000:.1f}W")
        print(f"  Util:  {util.gpu}%")
    pynvml.nvmlShutdown()
except pynvml.NVMLError:
    print("No NVIDIA GPU found")

# AMD
try:
    count = pyamdgpuinfo.detect_gpus()
    for i in range(count):
        gpu = pyamdgpuinfo.get_gpu(i)
        print(f"[AMD] {gpu.name}")
        print(f"  VRAM:  {gpu.query_vram_usage() // 1024**2} / {gpu.memory_info['vram_size'] // 1024**2} MB")
        print(f"  Temp:  {gpu.query_temperature()}°C")
        print(f"  Power: {gpu.query_power():.1f}W")
        print(f"  Util:  {gpu.query_load() * 100:.1f}%")
except Exception as e:
    print(f"No AMD GPU found: {e}")