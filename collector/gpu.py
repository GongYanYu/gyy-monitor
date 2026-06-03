"""GPU 硬件数据采集器（NVIDIA GPU，通过 pynvml）。"""

from collector.base import BaseCollector


class GpuCollector(BaseCollector):
    """采集 GPU 占用率、温度、功耗、显存、风扇转速。

    仅支持 NVIDIA GPU（通过 NVML）。
    非 NVIDIA 设备或 NVML 不可用时，所有指标返回 None。
    """

    def __init__(self):
        self._nvml_available = False
        self._handle = None
        try:
            import pynvml
            pynvml.nvmlInit()
            # 获取第一个 GPU
            count = pynvml.nvmlDeviceGetCount()
            if count > 0:
                self._handle = pynvml.nvmlDeviceGetHandleByIndex(0)
                self._nvml_available = True
        except Exception:
            self._nvml_available = False

    @property
    def available_metrics(self) -> list[str]:
        return ["gpu_usage", "gpu_temp", "gpu_power", "gpu_vram", "gpu_fan"]

    def collect(self) -> dict:
        if not self._nvml_available:
            return {
                "gpu_usage": None, "gpu_temp": None,
                "gpu_power": None, "gpu_vram": None, "gpu_fan": None,
            }

        import pynvml

        result = {}

        # GPU 占用率
        try:
            util = pynvml.nvmlDeviceGetUtilizationRates(self._handle)
            result["gpu_usage"] = round(util.gpu, 1)
        except Exception:
            result["gpu_usage"] = None

        # GPU 温度
        try:
            temp = pynvml.nvmlDeviceGetTemperature(
                self._handle, pynvml.NVML_TEMPERATURE_GPU
            )
            result["gpu_temp"] = round(temp, 1)
        except Exception:
            result["gpu_temp"] = None

        # GPU 功耗（W）
        try:
            power = pynvml.nvmlDeviceGetPowerUsage(self._handle)
            # pynvml 返回毫瓦，转换为瓦
            result["gpu_power"] = round(power / 1000.0, 1)
        except Exception:
            result["gpu_power"] = None

        # 显存占用率
        try:
            mem_info = pynvml.nvmlDeviceGetMemoryInfo(self._handle)
            vram_pct = (mem_info.used / mem_info.total) * 100
            result["gpu_vram"] = round(vram_pct, 1)
        except Exception:
            result["gpu_vram"] = None

        # GPU 风扇转速
        try:
            fan = pynvml.nvmlDeviceGetFanSpeed(self._handle)
            result["gpu_fan"] = fan
        except Exception:
            result["gpu_fan"] = None

        return result

    def shutdown(self):
        """释放 NVML 资源。"""
        if self._nvml_available:
            try:
                import pynvml
                pynvml.nvmlShutdown()
            except Exception:
                pass
