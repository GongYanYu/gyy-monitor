"""FPS 帧率采集器（占位实现，后续通过 ETW/DXGI 完善）。"""

from collector.base import BaseCollector


class FpsCollector(BaseCollector):
    """采集当前屏幕帧率。

    首版为占位实现，返回 None。
    后续通过 Windows ETW 监听 DXGI Present 事件获取真实帧率。
    """

    @property
    def available_metrics(self) -> list[str]:
        return ["fps", "fps_1pct_low"]

    def collect(self) -> dict:
        return {
            "fps": None,
            "fps_1pct_low": None,
        }

    def start(self):
        """启动 ETW 监听（占位，后续实现）。"""
        pass

    def stop(self):
        """停止 ETW 监听（占位，后续实现）。"""
        pass
