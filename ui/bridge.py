"""Python <-> JavaScript 桥接 API。

通过 pywebview 的 api 机制暴露给前端调用。
"""

import json
import os
import sys
import winreg
from pathlib import Path

def log_debug(msg):
    try:
        with open(r"d:\MyProjects\gyy-monitor\debug.log", "a", encoding="utf-8") as f:
            f.write(msg + "\n")
    except Exception:
        pass

# 针对 PyQt5 的全局跨线程自定义事件类
CallbackEvent = None
try:
    from PyQt5.QtCore import QEvent
    class CallbackEvent(QEvent):
        def __init__(self, callback):
            super().__init__(QEvent.Type(QEvent.User + 1234))
            self.callback = callback
except ImportError:
    pass


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
        self._on_quit_callback = None   # 退出应用回调
        self._settings_window = None    # 设置窗口实例

        # 线程安全辅助器 (针对 PyQt5 GUI 线程限制，将在 shown 事件中由主 GUI 线程初始化)
        self._qt_helper = None

    def bind_window(self, window):
        """绑定 pywebview 窗口实例。"""
        self._window = window

    def init_qt_helper(self):
        """注入主事件循环的回调处理器。"""
        log_debug(f"[Bridge] init_qt_helper 被调用, self._qt_helper 存在={self._qt_helper is not None}")
        if self._qt_helper:
            log_debug("[Bridge] _qt_helper 已存在，跳过初始化")
            return
        native = self._window.native
        if not native:
            log_debug("[Bridge] native 窗口为 None，注入失败")
            return
        try:
            from PyQt5.QtCore import QEvent
            # 保存原有的 customEvent 处理器
            old_custom_event = native.customEvent if hasattr(native, 'customEvent') else None
            
            def new_custom_event(event):
                if event.type() == QEvent.User + 1234:
                    try:
                        event.callback()
                    except Exception as err:
                        log_debug(f"[customEvent] 执行回调失败: {err}")
                    return True
                if old_custom_event:
                    return old_custom_event(event)
                return False
                
            native.customEvent = new_custom_event
            self._qt_helper = True
            log_debug("[Bridge] 成功向 Qt 主窗口注入 customEvent 处理器！")
        except Exception as e:
            log_debug(f"[Bridge] 注入 customEvent 处理器失败: {e}")

    def set_on_config_changed(self, callback):
        """设置配置变更回调。"""
        self._on_config_changed = callback

    def set_on_quit(self, callback):
        """设置退出应用回调。"""
        self._on_quit_callback = callback

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

            # 如果是 autostart 配置项，同步修改注册表
            if key == "autostart":
                self._update_autostart_registry(value)

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

            # 如果包含 autostart，更新自启注册表
            if "autostart" in patch:
                self._update_autostart_registry(patch["autostart"])

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

    def show_context_menu(self):
        """显示原生右键菜单（通过主线程机制防止跨线程 GUI 调用导致 Python 崩溃）。"""
        log_debug(f"[Bridge] show_context_menu 触发, _window 存在={self._window is not None}")
        if not self._window:
            return

        native = self._window.native
        if not native:
            log_debug("[Bridge] native 窗口对象为 None，直接返回")
            return

        native_type = type(native).__name__
        log_debug(f"[Bridge] native 窗口类型: {native_type}")

        # 1. PyQt5 / PySide2 QMainWindow (pywebview 类名为 BrowserView)
        if "BrowserView" in native_type or "QMainWindow" in native_type or hasattr(native, 'winId'):
            log_debug(f"[Bridge] 匹配到 Qt 后端，_qt_helper 存在={self._qt_helper is not None}")
            if self._qt_helper and CallbackEvent:
                log_debug("[Bridge] 通过 QCoreApplication.postEvent 将回调发送到主线程")
                from PyQt5.QtCore import QCoreApplication
                event = CallbackEvent(self._show_qt_menu)
                QCoreApplication.postEvent(native, event)
            else:
                log_debug("[Bridge] 辅助器或事件类未就绪，尝试在当前线程执行 _show_qt_menu (可能会崩溃)")
                self._show_qt_menu()

        # 2. WinForms Form
        elif "Form" in native_type:
            try:
                import clr
                clr.AddReference("System.Windows.Forms")
                from System import Action
                # WinForms 跨线程调用需使用 Invoke
                if native.InvokeRequired:
                    native.Invoke(Action(self._show_winforms_menu))
                else:
                    self._show_winforms_menu()
            except Exception as e:
                print(f"[Bridge] 跨线程调用 WinForms 菜单失败: {e}")

    def _show_qt_menu(self):
        """在 Qt 主 GUI 线程渲染并弹出右键菜单。"""
        log_debug("[Bridge] _show_qt_menu 开始在当前线程执行...")
        try:
            from PyQt5.QtWidgets import QMenu, QAction
            from PyQt5.QtGui import QCursor

            menu = QMenu()
            log_debug("[Bridge] QMenu 实例化成功")

            # 点击穿透
            click_through = self._config["window"].get("click_through", False)
            txt_through = "🔲 关闭穿透" if click_through else "🔳 开启穿透"
            act_through = QAction(txt_through, menu)
            act_through.triggered.connect(lambda: self.toggle_click_through(not click_through))
            menu.addAction(act_through)

            menu.addSeparator()

            # 风格子菜单
            style_menu = menu.addMenu("🎨 风格")
            curr_style = self._config["display"].get("style", "minimal")
            for s, label in [("minimal", "极简数字"), ("glass", "暗色玻璃"), ("hacker", "终端黑客")]:
                act = QAction(label, style_menu, checkable=True, checked=(s == curr_style))
                act.triggered.connect(lambda checked, val=s: self.set_config("display.style", val))
                style_menu.addAction(act)

            # 布局子菜单
            layout_menu = menu.addMenu("📐 布局")
            curr_layout = self._config["display"].get("layout", "horizontal")
            for l, label in [("horizontal", "水平"), ("vertical", "垂直"), ("grid", "网格")]:
                act = QAction(label, layout_menu, checkable=True, checked=(l == curr_layout))
                act.triggered.connect(lambda checked, val=l: self.set_config("display.layout", val))
                layout_menu.addAction(act)

            menu.addSeparator()

            # 设置
            act_settings = QAction("⚙️ 设置...", menu)
            act_settings.triggered.connect(self.open_settings_window)
            menu.addAction(act_settings)

            # 隐藏窗口
            act_hide = QAction("👁 隐藏窗口", menu)
            act_hide.triggered.connect(self.toggle_visible)
            menu.addAction(act_hide)

            menu.addSeparator()

            # 退出
            act_quit = QAction("❌ 退出", menu)
            act_quit.triggered.connect(self.quit_app)
            menu.addAction(act_quit)

            menu.exec_(QCursor.pos())
        except Exception as e:
            print(f"[Bridge] 弹出 PyQt5 右键菜单失败: {e}")

    def _show_winforms_menu(self):
        """在 WinForms 主 GUI 线程渲染并弹出右键菜单。"""
        try:
            import clr
            clr.AddReference("System.Windows.Forms")
            import System.Windows.Forms as WinForms

            menu = WinForms.ContextMenuStrip()

            # 点击穿透
            click_through = self._config["window"].get("click_through", False)
            txt_through = "🔲 关闭穿透" if click_through else "🔳 开启穿透"
            item_through = menu.Items.Add(txt_through)
            item_through.Click += lambda s, e: self.toggle_click_through(not click_through)

            menu.Items.Add(WinForms.ToolStripSeparator())

            # 风格
            item_style = WinForms.ToolStripMenuItem("🎨 风格")
            curr_style = self._config["display"].get("style", "minimal")
            for s, label in [("minimal", "极简数字"), ("glass", "暗色玻璃"), ("hacker", "终端黑客")]:
                sub = WinForms.ToolStripMenuItem(label)
                sub.Checked = (s == curr_style)
                sub.Click += lambda s_sender, e_args, val=s: self.set_config("display.style", val)
                item_style.DropDownItems.Add(sub)
            menu.Items.Add(item_style)

            # 布局
            item_layout = WinForms.ToolStripMenuItem("📐 布局")
            curr_layout = self._config["display"].get("layout", "horizontal")
            for l, label in [("horizontal", "水平"), ("vertical", "垂直"), ("grid", "网格")]:
                sub = WinForms.ToolStripMenuItem(label)
                sub.Checked = (l == curr_layout)
                sub.Click += lambda s_sender, e_args, val=l: self.set_config("display.layout", val)
                item_layout.DropDownItems.Add(sub)
            menu.Items.Add(item_layout)

            menu.Items.Add(WinForms.ToolStripSeparator())

            # 设置
            item_settings = menu.Items.Add("⚙️ 设置...")
            item_settings.Click += lambda s, e: self.open_settings_window()

            # 隐藏窗口
            item_hide = menu.Items.Add("👁 隐藏窗口")
            item_hide.Click += lambda s, e: self.toggle_visible()

            menu.Items.Add(WinForms.ToolStripSeparator())

            # 退出
            item_quit = menu.Items.Add("❌ 退出")
            item_quit.Click += lambda s, e: self.quit_app()

            pos = WinForms.Control.MousePosition
            menu.Show(pos)
        except Exception as e:
            print(f"[Bridge] 弹出 WinForms 右键菜单失败: {e}")

    def open_settings_window(self):
        """打开设置窗口。"""
        if hasattr(self, '_settings_window') and self._settings_window:
            try:
                self._settings_window.show()
                return
            except Exception:
                self._settings_window = None

        import webview
        web_dir = Path(__file__).parent.parent / "web"
        settings_html = str(web_dir / "settings.html")

        always_on_top = self._config["window"].get("always_on_top", True)

        self._settings_window = webview.create_window(
            title="gyy-monitor 设置",
            url=settings_html,
            width=550,
            height=650,
            resizable=True,
            on_top=always_on_top,
            js_api=self,
        )
        self._settings_window.events.closed += self._on_settings_closed

    def close_settings_window(self):
        """主动关闭设置窗口。"""
        if hasattr(self, '_settings_window') and self._settings_window:
            try:
                self._settings_window.destroy()
            except Exception:
                pass
            self._settings_window = None

    def _on_settings_closed(self):
        self._settings_window = None

    def quit_app(self):
        """安全退出应用。"""
        if self._on_quit_callback:
            self._on_quit_callback()
        else:
            os._exit(0)

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

    def _update_autostart_registry(self, enabled: bool):
        """修改 Windows 注册表以设置/取消开机自启。"""
        try:
            key_path = r"Software\Microsoft\Windows\CurrentVersion\Run"
            if getattr(sys, 'frozen', False):
                # 编译打包后的 EXE 路径
                exe_path = sys.executable
            else:
                # 脚本运行路径：直接获取 main.py 的绝对路径
                main_py = Path(__file__).parent.parent / "main.py"
                exe_path = f'"{sys.executable}" "{main_py.resolve()}"'

            key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path, 0, winreg.KEY_SET_VALUE)
            if enabled:
                winreg.SetValueEx(key, "GyyMonitor", 0, winreg.REG_SZ, exe_path)
                print(f"[Registry] 已添加开机自启: {exe_path}")
            else:
                try:
                    winreg.DeleteValue(key, "GyyMonitor")
                    print("[Registry] 已取消开机自启")
                except FileNotFoundError:
                    pass
            winreg.CloseKey(key)
        except Exception as e:
            print(f"[Registry] 写入注册表自启项失败: {e}")

    def _set_click_through(self, enabled: bool):
        """通过 Windows API 设置 WS_EX_TRANSPARENT 样式。"""
        try:
            import ctypes
            import ctypes.wintypes

            GWL_EXSTYLE = -20
            WS_EX_TRANSPARENT = 0x00000020

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
