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

        # 监听 shown 事件，在窗口显示后设置 WebView2 背景透明
        self._window.events.shown += self._set_webview_transparent

        if win_cfg.get("click_through", False):
            self._bridge.toggle_click_through(True)

    def _set_webview_transparent(self, *args, **kwargs):
        """设置 WebView2 背景透明。

        pywebview transparent=True 在 WinForms 下只设置了窗口样式，
        WebView2 控件自身以及主窗体也需要设置背景色为全透明。
        若是 Qt (PyQt5) 后端，pywebview 已原生支持透明，直接跳过。
        """
        try:
            # 检查 native 是否是 WinForms Form 窗口
            native = self._window.native
            if not native:
                return
            native_type_name = type(native).__name__
            if "Form" not in native_type_name:
                return

            import clr
            clr.AddReference("System.Drawing")
            import System.Drawing
            import ctypes

            # 获取 WinForms BrowserForm 并设置背景色为黑色（DWM 开启模糊后黑色会被透明化）
            browser_form = native
            browser_form.BackColor = System.Drawing.Color.Black

            # 获取 WebView2 控件并将其背景色设为透明色
            webview_control = browser_form.Controls[0]
            webview_control.DefaultBackgroundColor = System.Drawing.Color.Transparent

            # 获取窗体句柄 HWND 并使用 DWM API 开启全窗口的背景磨砂透明
            hwnd = browser_form.Handle.ToInt64()

            class DWM_BLURBEHIND(ctypes.Structure):
                _fields_ = [
                    ("dwFlags", ctypes.c_ulong),
                    ("fEnable", ctypes.c_bool),
                    ("hRgnBlur", ctypes.c_void_p),
                    ("fTransitionOnMaximized", ctypes.c_bool),
                ]

            dwmapi = ctypes.windll.dwmapi
            bb = DWM_BLURBEHIND()
            bb.dwFlags = 1  # DWM_BB_ENABLE
            bb.fEnable = True
            bb.hRgnBlur = ctypes.c_void_p(0)

            dwmapi.DwmEnableBlurBehindWindow(ctypes.c_void_p(hwnd), ctypes.byref(bb))
        except Exception as e:
            print(f"[Window] 设置 WebView2 背景透明失败: {e}")

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
