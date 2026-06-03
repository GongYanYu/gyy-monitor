"""内存数据采集器。"""

import psutil
from collector.base import BaseCollector


class MemoryCollector(BaseCollector):
    """采集系统内存占用。"""

    @property
    def available_metrics(self) -> list[str]:
        return ["ram_usage", "ram_used", "ram_total"]

    def collect(self) -> dict:
        result = {}

        try:
            mem = psutil.virtual_memory()
            result["ram_usage"] = round(mem.percent, 1)
            result["ram_used"] = round(mem.used / (1024 ** 3), 1)
            result["ram_total"] = round(mem.total / (1024 ** 3), 1)
        except Exception:
            result["ram_usage"] = None
            result["ram_used"] = None
            result["ram_total"] = None

        return result
