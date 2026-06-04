"""CPU 硬件数据采集器。"""

import psutil
from collector.base import BaseCollector


class CpuCollector(BaseCollector):
    """采集 CPU 占用率、温度、功耗、风扇转速。"""

    def __init__(self):
        # 预调用一次以建立基线，后续 collect() 使用 interval=None 非阻塞获取差值
        psutil.cpu_percent(interval=None)

    @property
    def available_metrics(self) -> list[str]:
        return ["cpu_usage", "cpu_temp", "cpu_power", "cpu_fan"]

    def collect(self) -> dict:
        result = {}

        # CPU 占用率（非阻塞，返回自上次调用以来的差值）
        try:
            result["cpu_usage"] = round(psutil.cpu_percent(interval=None), 1)
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

        在 Linux/FreeBSD 下优先通过 psutil sensors_temperatures() 获取，
        在 Windows 下通过 WMI 性能计数器获取（无需管理员权限）。
        """
        import sys
        if sys.platform == "win32":
            return self._get_cpu_temp_windows()

        try:
            temps = psutil.sensors_temperatures()
            if not temps:
                return None

            for label in ("coretemp", "k10temp", "cpu_thermal"):
                if label in temps:
                    entries = temps[label]
                    if entries:
                        return round(max(e.current for e in entries if e.current is not None), 1)

            # 兜底：取第一个传感器的最高值
            first = next(iter(temps.values()))
            if first:
                return round(max(e.current for e in first if e.current is not None), 1)

            return None
        except Exception:
            return None

    def _get_cpu_temp_windows(self) -> float | None:
        """Windows 下免管理员权限获取 CPU/系统热力分区温度。"""
        import subprocess
        # 1. 优先通过 wmic 获取 HighPrecisionTemperature (分度为 0.1 Kelvin)
        try:
            out = subprocess.check_output(
                'wmic path Win32_PerfFormattedData_Counters_ThermalZoneInformation get HighPrecisionTemperature',
                shell=True, text=True, stderr=subprocess.DEVNULL
            )
            temps = []
            for line in out.strip().splitlines():
                line = line.strip()
                if line and line.isdigit():
                    val = float(line)
                    c = (val / 10.0) - 273.15
                    if 0 < c < 150:
                        temps.append(c)
            if temps:
                return round(max(temps), 1)
        except Exception:
            pass

        # 2. 备选通过 wmic 获取 standard Temperature (分度为 1 Kelvin)
        try:
            out = subprocess.check_output(
                'wmic path Win32_PerfFormattedData_Counters_ThermalZoneInformation get Temperature',
                shell=True, text=True, stderr=subprocess.DEVNULL
            )
            temps = []
            for line in out.strip().splitlines():
                line = line.strip()
                if line and line.isdigit():
                    val = float(line)
                    c = val - 273.15
                    if 0 < c < 150:
                        temps.append(c)
            if temps:
                return round(max(temps), 1)
        except Exception:
            pass

        # 3. 备选通过 powershell 获取
        try:
            cmd = 'powershell -NoProfile -Command "Get-CimInstance -Namespace root/cimv2 -ClassName Win32_PerfFormattedData_Counters_ThermalZoneInformation | Select-Object -ExpandProperty HighPrecisionTemperature"'
            out = subprocess.check_output(cmd, shell=True, text=True, stderr=subprocess.DEVNULL)
            temps = []
            for line in out.strip().splitlines():
                line = line.strip()
                if line and line.isdigit():
                    val = float(line)
                    c = (val / 10.0) - 273.15
                    if 0 < c < 150:
                        temps.append(c)
            if temps:
                return round(max(temps), 1)
        except Exception:
            pass

        return None

    def _get_cpu_power(self) -> float | None:
        """获取 CPU 封装功耗（W）。

        在 Linux/FreeBSD 下通过 psutil sensors_power() 或 RAPL 获取，
        在 Windows 下通过 WMI 能量计数器（RAPL_Package0_PKG）获取。
        """
        import sys
        if sys.platform == "win32":
            return self._get_cpu_power_windows()

        try:
            power = psutil.sensors_power()
            if not power:
                return None

            # RAPL 常见标签：package-0（Intel）、package、cpu
            for label in ("package-0", "package", "cpu"):
                if label in power:
                    entries = power[label]
                    if entries:
                        return round(sum(e.current for e in entries if e.current is not None), 1)

            return None
        except Exception:
            return None

    def _get_cpu_power_windows(self) -> float | None:
        """Windows 下免管理员权限获取 CPU 封装功耗 (W)。"""
        import subprocess
        try:
            out = subprocess.check_output(
                'wmic path Win32_PerfFormattedData_PowerMeterCounter_EnergyMeter get Name,Power',
                shell=True, text=True, stderr=subprocess.DEVNULL
            )
            for line in out.strip().splitlines():
                parts = line.strip().split()
                if len(parts) >= 2:
                    name, power_str = parts[0], parts[1]
                    if "RAPL_Package0_PKG" in name and power_str.isdigit():
                        return round(float(power_str) / 1000.0, 1)
        except Exception:
            pass

        # 备选：如果找不到 PKG，取所有的最大值
        try:
            out = subprocess.check_output(
                'wmic path Win32_PerfFormattedData_PowerMeterCounter_EnergyMeter get Power',
                shell=True, text=True, stderr=subprocess.DEVNULL
            )
            powers = []
            for line in out.strip().splitlines():
                line = line.strip()
                if line and line.isdigit():
                    powers.append(float(line))
            if powers:
                return round(max(powers) / 1000.0, 1)
        except Exception:
            pass

        return None

    def _get_cpu_fan(self) -> int | None:
        """获取 CPU 风扇转速（RPM）。

        在 Linux/FreeBSD 下通过 psutil sensors_fans() 获取，
        在 Windows 下通过 LHM/OHM WMI 接口获取（免管理员权限）。
        """
        import sys
        if sys.platform == "win32":
            return self._get_cpu_fan_windows()

        try:
            fans = psutil.sensors_fans()
            if not fans:
                return None

            # 优先查找 CPU 风扇
            for label in ("cpu_fan", "cpufan", "cpu"):
                if label in fans:
                    entries = fans[label]
                    if entries:
                        speeds = [e.current for e in entries if e.current is not None]
                        if speeds:
                            return max(speeds)

            # 兜底：取所有风扇中的最大值
            for label in fans:
                entries = fans[label]
                if entries:
                    speeds = [e.current for e in entries if e.current is not None]
                    if speeds:
                        return max(speeds)

            return None
        except Exception:
            return None

    def _get_cpu_fan_windows(self) -> int | None:
        """Windows 下尝试通过 LibreHardwareMonitor 或 OpenHardwareMonitor 的 WMI 接口获取风扇转速。"""
        import subprocess
        # 1. 尝试 LibreHardwareMonitor WMI 命名空间
        try:
            out = subprocess.check_output(
                'wmic /namespace:\\\\root\\LibreHardwareMonitor PATH Sensor WHERE "SensorType=\'Fan\'" get Name,Value',
                shell=True, text=True, stderr=subprocess.DEVNULL
            )
            fans = []
            for line in out.strip().splitlines():
                parts = line.strip().split()
                if len(parts) >= 2:
                    val_str = parts[-1]
                    name = " ".join(parts[:-1])
                    try:
                        val = float(val_str)
                        if "cpu" in name.lower():
                            return int(val)
                        fans.append(int(val))
                    except ValueError:
                        pass
            if fans:
                return max(fans)
        except Exception:
            pass

        # 2. 尝试 OpenHardwareMonitor WMI 命名空间
        try:
            out = subprocess.check_output(
                'wmic /namespace:\\\\root\\OpenHardwareMonitor PATH Sensor WHERE "SensorType=\'Fan\'" get Name,Value',
                shell=True, text=True, stderr=subprocess.DEVNULL
            )
            fans = []
            for line in out.strip().splitlines():
                parts = line.strip().split()
                if len(parts) >= 2:
                    val_str = parts[-1]
                    name = " ".join(parts[:-1])
                    try:
                        val = float(val_str)
                        if "cpu" in name.lower():
                            return int(val)
                        fans.append(int(val))
                    except ValueError:
                        pass
            if fans:
                return max(fans)
        except Exception:
            pass

        return None
