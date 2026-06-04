"""Python <-> JavaScript 桥接 API。

通过 pywebview 的 api 机制暴露给前端调用。
"""

import json
import os
from pathlib import Path


class Bridge:
    """暴露给前端 JS 的 API 类。

    pywebview 会自动将此类的方法通过 window.pywebview.api 暴露给 JS。
    """

    def __init__(self, config_path: str, config: dict):
        self._config_path = config_path
        self._config = config
        self._window = None          # pywebview window 实例，由外部设置
        self._metrics_snapshot = {}  # 采集线程写入的最新数据
        self._on_config_changed = None  # 配置变更回调

    def bind_window(self, window):
        """绑定 pywebview 窗口实例。"""
        self._window = window

    def set_on_config_changed(self, callback):
        """设置配置变更回调。"""
        self._on_config_changed = callback

    def update_metrics_snapshot(self, data: dict):
        """采集线程调用，更新最新指标快照。"""
        self._metrics_snapshot = data

    # === 前端可调用的 API 方法 ===

    def get_config(self) -> dict:
        """返回完整配置。"""
        return self._config

    def get_metrics(self) -> dict:
        """返回当前指标快照。"""
        return self._metrics_snapshot

    def get_available_metrics(self) -> list[str]:
        """返回所有可用的指标 ID 列表。"""
        return [m["id"] for m in self._config.get("metrics", [])]

    def set_config(self, key: str, value) -> bool:
        """修改单个配置项（支持点号分隔的嵌套 key）。

        示例: set_config('display.style', 'glass')
        """
        try:
            keys = key.split(".")
            obj = self._config
            for k in keys[:-1]:
                obj = obj[k]
            obj[keys[-1]] = value
            self._save_config()
            return True
        except Exception as e:
            print(f"[Bridge] set_config 失败: {e}")
            return False

    def set_config_bulk(self, patch: dict) -> bool:
        """批量修改配置（递归合并顶层 key）。"""
        try:
            for k, v in patch.items():
                if k in self._config and isinstance(self._config[k], dict) and isinstance(v, dict):
                    self._config[k].update(v)
                else:
                    self._config[k] = v
            self._save_config()
            return True
        except Exception as e:
            print(f"[Bridge] set_config_bulk 失败: {e}")
            return False

    def toggle_click_through(self, enabled: bool) -> bool:
        """切换点击穿透模式。"""
        try:
            if self._window:
                self._set_click_through(enabled)
            return True
        except Exception as e:
            print(f"[Bridge] toggle_click_through 失败: {e}")
            return False

    def move_window(self, dx: int, dy: int) -> bool:
        """移动窗口相对位置。"""
        try:
            if self._window:
                x = self._window.x + dx
                y = self._window.y + dy
                self._window.move(x, y)
            return True
        except Exception:
            return False

    def toggle_visible(self) -> bool:
        """切换窗口显示/隐藏。"""
        try:
            if self._window:
                if self._window.visible:
                    self._window.hide()
                else:
                    self._window.show()
            return True
        except Exception as e:
            print(f"[Bridge] toggle_visible 失败: {e}")
            return False

    def open_config_file(self) -> bool:
        """用系统默认编辑器打开配置文件。"""
        try:
            os.startfile(self._config_path)
            return True
        except Exception as e:
            print(f"[Bridge] open_config_file 失败: {e}")
            return False

    def read_file(self, path: str) -> str | None:
        """读取文件内容（供前端读取自定义 CSS 文件）。"""
        try:
            return Path(path).read_text(encoding="utf-8")
        except Exception as e:
            print(f"[Bridge] read_file 失败: {e}")
            return None

    # === 内部方法 ===

    def _save_config(self):
        """保存配置到文件，并通知配置变更回调。"""
        try:
            with open(self._config_path, "w", encoding="utf-8") as f:
                json.dump(self._config, f, indent=2, ensure_ascii=False)
            if self._on_config_changed:
                self._on_config_changed(self._config)
        except Exception as e:
            print(f"[Bridge] 保存配置失败: {e}")

    def _set_click_through(self, enabled: bool):
        """通过 Windows API 设置 WS_EX_TRANSPARENT 样式。"""
        try:
            import ctypes
            import ctypes.wintypes

            GWL_EXSTYLE = -20
            WS_EX_TRANSPARENT = 0x00000020
            WS_EX_LAYERED = 0x00080000
            WS_EX_TOOLWINDOW = 0x00000080

            hwnd = None
            if hasattr(self._window, 'native') and self._window.native:
                native = self._window.native
                # WinForms
                if hasattr(native, 'Handle'):
                    hwnd = native.Handle.ToInt64()
                # Qt
                elif hasattr(native, 'winId'):
                    hwnd = int(native.winId())

            if not hwnd:
                # 尝试通过 ctypes 查找窗口
                user32 = ctypes.windll.user32
                hwnd = user32.FindWindowW(None, "gyy-monitor")
                if not hwnd:
                    return

            user32 = ctypes.windll.user32
            ex_style = user32.GetWindowLongW(hwnd, GWL_EXSTYLE)
            if enabled:
                ex_style |= WS_EX_TRANSPARENT
            else:
                ex_style &= ~WS_EX_TRANSPARENT
            user32.SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style)
        except Exception as e:
            print(f"[Bridge] _set_click_through 失败: {e}")
