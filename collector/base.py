"""采集器基类，所有硬件数据采集器继承此类。"""

from abc import ABC, abstractmethod


class BaseCollector(ABC):
    """所有采集器继承此类，统一 collect() 接口。"""

    @abstractmethod
    def collect(self) -> dict:
        """返回指标字典。

        Returns:
            dict: 如 {'cpu_usage': 42.5, 'cpu_temp': 56}。
                  不可用的值设为 None。
        """
        ...

    @property
    @abstractmethod
    def available_metrics(self) -> list[str]:
        """返回该采集器能提供的指标 ID 列表。

        Returns:
            list[str]: 指标 ID 列表。
        """
        ...
