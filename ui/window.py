"""pywebview 窗口管理模块。

负责创建、配置和管理悬浮窗窗口。
"""

import os
import webview
from pathlib import Path


class MonitorWindow:
    """管理悬浮窗的 pywebview 窗口。"""

    def __init__(self, config: dict, bridge):
        self._config = config
        self._bridge = bridge
        self._window = None

    def create(self):
        """创建并显示悬浮窗窗口。"""
        win_cfg = self._config["window"]

        # 计算 web 目录的绝对路径
        web_dir = Path(__file__).parent.parent / "web"
        html_path = str(web_dir / "index.html")

        # 创建窗口
        self._window = webview.create_window(
            title="gyy-monitor",
            url=html_path,
            width=win_cfg.get("width", 400),
            height=win_cfg.get("height", 60),
            x=win_cfg.get("x"),
            y=win_cfg.get("y"),
            frameless=win_cfg.get("frameless", True),
            always_on_top=win_cfg.get("always_on_top", True),
            transparent=True,
            background_color="#00000000",  # 完全透明背景
            easy_drag=False,               # 由 JS 端处理拖动
            on_top=True,
            js_api=self._bridge,           # 暴露 Bridge 给 JS
        )

        self._bridge.bind_window(self._window)

        # 应用点击穿透设置
        if win_cfg.get("click_through", False):
            self._bridge._set_click_through(True)

    def push_metrics(self, data: dict):
        """推送指标数据到前端。

        Args:
            data: 指标字典，如 {'cpu_usage': 42.5, ...}
        """
        if not self._window:
            return
        # 转成 JSON 字符串注入 JS
        import json
        json_data = json.dumps(data, ensure_ascii=False)
        try:
            self._window.evaluate_js(f"window.updateMetrics && window.updateMetrics({json_data})")
        except Exception:
            pass

    def push_config(self, config: dict):
        """推送配置到前端（热加载）。

        Args:
            config: 完整配置字典。
        """
        if not self._window:
            return
        import json
        json_cfg = json.dumps(config, ensure_ascii=False)
        try:
            self._window.evaluate_js(f"window.updateConfig && window.updateConfig({json_cfg})")
        except Exception:
            pass

    def show(self):
        """显示窗口。"""
        if self._window:
            try:
                self._window.show()
            except Exception:
                pass

    def hide(self):
        """隐藏窗口。"""
        if self._window:
            try:
                self._window.hide()
            except Exception:
                pass

    def toggle(self):
        """切换窗口显隐。"""
        if self._window:
            try:
                if self._window.visible:
                    self._window.hide()
                else:
                    self._window.show()
            except Exception:
                pass

    def destroy(self):
        """销毁窗口。"""
        if self._window:
            try:
                self._window.destroy()
            except Exception:
                pass
            self._window = None
