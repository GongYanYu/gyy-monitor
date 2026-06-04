"""pywebview 窗口管理模块。"""

import webview
from pathlib import Path


class MonitorWindow:
    """管理悬浮窗的 pywebview 窗口。"""

    def __init__(self, config: dict, bridge):
        self._config = config
        self._bridge = bridge
        self._window = None

    def create(self):
        win_cfg = self._config["window"]
        web_dir = Path(__file__).parent.parent / "web"
        html_path = str(web_dir / "index.html")

        self._window = webview.create_window(
            title="gyy-monitor",
            url=html_path,
            width=win_cfg.get("width", 400),
            height=win_cfg.get("height", 60),
            x=win_cfg.get("x"),
            y=win_cfg.get("y"),
            frameless=win_cfg.get("frameless", True),
            on_top=win_cfg.get("always_on_top", True),
            transparent=True,
            easy_drag=False,
            js_api=self._bridge,
        )

        self._bridge.bind_window(self._window)

        # WebView2 透明：设置背景色 alpha=0
        self._set_webview_transparent()

        if win_cfg.get("click_through", False):
            self._bridge.toggle_click_through(True)

    def _set_webview_transparent(self):
        """设置 WebView2 背景透明。

        pywebview transparent=True 只设置了窗口样式，
        WebView2 控制器自身也需要设置背景色为全透明。
        """
        try:
            # pywebview 6.x 内部: _edge_holder 持有 EdgeChrome 实例
            edge = getattr(self._window, '_edge_holder', None)
            if edge is None:
                return
            # CoreWebView2Controller2.put_DefaultBackgroundColor(0) = 全透明
            ctrl = edge.GetCoreWebView2Controller2()
            ctrl.put_DefaultBackgroundColor(0)  # 0 = RGBA(0,0,0,0)
        except Exception:
            pass

    def push_metrics(self, data: dict):
        if not self._window:
            return
        import json
        try:
            self._window.evaluate_js(
                "window.updateMetrics && window.updateMetrics("
                + json.dumps(data, ensure_ascii=False) + ")"
            )
        except Exception:
            pass

    def push_config(self, config: dict):
        if not self._window:
            return
        import json
        try:
            self._window.evaluate_js(
                "window.updateConfig && window.updateConfig("
                + json.dumps(config, ensure_ascii=False) + ")"
            )
        except Exception:
            pass

    def show(self):
        if self._window:
            try: self._window.show()
            except Exception: pass

    def hide(self):
        if self._window:
            try: self._window.hide()
            except Exception: pass

    def toggle(self):
        if self._window:
            try:
                if self._window.visible:
                    self._window.hide()
                else:
                    self._window.show()
            except Exception:
                pass

    def destroy(self):
        if self._window:
            try: self._window.destroy()
            except Exception: pass
            self._window = None
