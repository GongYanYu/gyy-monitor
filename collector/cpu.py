"""CPU 硬件数据采集器。"""

import psutil
from collector.base import BaseCollector


class CpuCollector(BaseCollector):
    """采集 CPU 占用率、温度、功耗、风扇转速。"""

    @property
    def available_metrics(self) -> list[str]:
        return ["cpu_usage", "cpu_temp", "cpu_power", "cpu_fan"]

    def collect(self) -> dict:
        result = {}

        # CPU 占用率（非阻塞，取 0.5s 间隔）
        try:
            result["cpu_usage"] = round(psutil.cpu_percent(interval=0.5), 1)
        except Exception:
            result["cpu_usage"] = None

        # CPU 温度（通过 psutil sensors_temperatures）
        result["cpu_temp"] = self._get_cpu_temp()

        # CPU 功耗（通过 psutil sensors_power 或 RAPL）
        result["cpu_power"] = self._get_cpu_power()

        # CPU 风扇
        result["cpu_fan"] = self._get_cpu_fan()

        return result

    def _get_cpu_temp(self) -> float | None:
        """获取 CPU 封装温度。

        遍历 psutil.sensors_temperatures() 查找 CPU 相关传感器，
        优先返回 'coretemp' 或 'k10temp' 标签下的最高温度。
        """
        try:
            temps = psutil.sensors_temperatures()
            if not temps:
                return None

            for label in ("coretemp", "k10temp", "cpu_thermal"):
                if label in temps:
                    entries = temps[label]
                    if entries:
                        return round(max(e.current for e in entries if e.current), 1)

            # 兜底：取第一个传感器的最高值
            first = next(iter(temps.values()))
            if first:
                return round(max(e.current for e in first if e.current), 1)

            return None
        except Exception:
            return None

    def _get_cpu_power(self) -> float | None:
        """获取 CPU 封装功耗（W）。

        通过 psutil sensors_power() 或 Intel RAPL 接口获取。
        部分 AMD CPU 不可用，返回 None。
        """
        try:
            power = psutil.sensors_power()
            if not power:
                return None

            for label in ("coretemp", "cpu", "package"):
                if label in power:
                    entries = power[label]
                    if entries:
                        return round(sum(e.current for e in entries if e.current), 1)

            return None
        except Exception:
            return None

    def _get_cpu_fan(self) -> int | None:
        """获取 CPU 风扇转速（RPM）。

        遍历 psutil.sensors_fans() 查找风扇数据。
        """
        try:
            fans = psutil.sensors_fans()
            if not fans:
                return None

            for label in fans:
                entries = fans[label]
                if entries:
                    speeds = [e.current for e in entries if e.current]
                    if speeds:
                        return max(speeds)

            return None
        except Exception:
            return None
