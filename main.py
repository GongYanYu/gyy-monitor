"""gyy-monitor 主入口。

加载配置 → 启动采集线程 → 创建悬浮窗 → 运行系统托盘 → 进入消息循环。
"""

import json
import os
import sys
import threading
import time
from pathlib import Path

import webview

from collector.cpu import CpuCollector
from collector.gpu import GpuCollector
from collector.memory import MemoryCollector
from collector.fps import FpsCollector
from ui.bridge import Bridge
from ui.window import MonitorWindow
from ui.tray import SystemTray


def get_app_dir() -> Path:
    """获取应用数据目录（配置文件存放处）。"""
    if getattr(sys, 'frozen', False):
        # PyInstaller 打包后的路径
        base = Path(sys.executable).parent
    else:
        base = Path(__file__).parent
    return base


def load_config(config_path: str) -> dict:
    """加载配置文件，失败时返回内置默认配置。"""
    try:
        with open(config_path, "r", encoding="utf-8") as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError) as e:
        print(f"[Main] 加载配置失败: {e}，使用默认配置")
        return _default_config()


def _default_config() -> dict:
    """内置默认配置（配置文件丢失时的兜底）。"""
    return {
        "window": {
            "width": 400, "height": 60, "x": None, "y": None,
            "always_on_top": True, "click_through": False,
            "draggable": True, "opacity": 1.0, "frameless": True,
        },
        "display": {
            "style": "minimal", "layout": "horizontal",
            "custom_css": None, "font_size": 14, "gap": 18, "padding": 8,
        },
        "metrics": [
            {"id": "cpu_usage", "enabled": True, "label": "CPU", "unit": "%", "color": "#4fc3f7", "decimals": 0},
            {"id": "cpu_temp", "enabled": True, "label": "CPUT", "unit": "°C", "color": "#4fc3f7", "decimals": 0},
            {"id": "gpu_usage", "enabled": True, "label": "GPU", "unit": "%", "color": "#81c784", "decimals": 0},
            {"id": "gpu_temp", "enabled": True, "label": "GPUT", "unit": "°C", "color": "#81c784", "decimals": 0},
            {"id": "ram_usage", "enabled": True, "label": "RAM", "unit": "%", "color": "#ffb74d", "decimals": 0},
        ],
        "update_interval_ms": 1000,
        "autostart": False,
    }


class App:
    """应用主控制器，协调各模块。"""

    def __init__(self):
        self.app_dir = get_app_dir()
        self.config_path = str(self.app_dir / "config.json")
        self.config = load_config(self.config_path)

        # 初始化各模块
        self.bridge = Bridge(self.config_path, self.config)
        self.bridge.set_on_quit(self._on_quit)
        self.window = MonitorWindow(self.config, self.bridge)
        self.tray = SystemTray(
            on_toggle=self._on_toggle,
            on_open_config=self._on_open_config,
            on_style_change=self._on_style_change,
            on_quit=self._on_quit,
        )

        # 采集器
        self.collectors = [
            CpuCollector(),
            GpuCollector(),
            MemoryCollector(),
            FpsCollector(),
        ]
        self._collector_thread = None
        self._running = False

        # 配置热加载
        self.bridge.set_on_config_changed(self._on_config_reloaded)

    def start(self):
        """启动应用。"""
        self._running = True

        # 同步开机自启项到注册表
        self.bridge._update_autostart_registry(self.config.get("autostart", False))

        # 创建窗口
        self.window.create()

        # 启动采集线程
        self._collector_thread = threading.Thread(target=self._collect_loop, daemon=True)
        self._collector_thread.start()

        # 创建并启动托盘
        self.tray.create()
        tray_thread = self.tray.run_in_thread()

        # 尝试使用 PyQt5 作为 GUI 后端，以在 Windows 下完美支持透明背景
        gui_backend = None
        try:
            import PyQt5
            gui_backend = 'qt'
            print("[Main] 检测到 PyQt5 已安装，使用 qt 渲染后端以实现透明背景")
        except ImportError:
            print("[Main] 未检测到 PyQt5，使用默认 winforms 渲染后端")

        # 进入 pywebview 消息循环（阻塞）
        webview.start(gui=gui_backend, debug=False)

        # 清理
        self._running = False
        self.tray.stop()

    def stop(self):
        """停止应用。"""
        self._running = False
        for c in self.collectors:
            if hasattr(c, 'shutdown'):
                c.shutdown()
            if hasattr(c, 'stop'):
                c.stop()
        self.window.destroy()

    # === 内部方法 ===

    def _collect_loop(self):
        """采集线程主循环。"""
        interval_ms = self.config.get("update_interval_ms", 1000)
        interval_s = interval_ms / 1000.0

        while self._running:
            try:
                all_metrics = {}
                for collector in self.collectors:
                    try:
                        data = collector.collect()
                        all_metrics.update(data)
                    except Exception as e:
                        print(f"[Collector] {type(collector).__name__} 采集失败: {e}")

                self.bridge.update_metrics_snapshot(all_metrics)
                self.window.push_metrics(all_metrics)

            except Exception as e:
                print(f"[Collector] 采集循环异常: {e}")

            time.sleep(interval_s)

    def _on_toggle(self):
        """托盘：切换窗口显隐。"""
        try:
            self.window.toggle()
        except Exception:
            pass

    def _on_open_config(self):
        """托盘：打开配置文件。"""
        try:
            os.startfile(self.config_path)
        except Exception:
            pass

    def _on_style_change(self, style: str):
        """托盘：切换主题风格。"""
        self.config["display"]["style"] = style
        try:
            with open(self.config_path, "w", encoding="utf-8") as f:
                json.dump(self.config, f, indent=2, ensure_ascii=False)
            self.window.push_config(self.config)
        except Exception:
            pass

    def _on_quit(self):
        """托盘：退出应用。"""
        self.stop()
        self.tray.stop()
        os._exit(0)

    def _on_config_reloaded(self, config: dict):
        """配置热加载回调。"""
        self.config = config

        # 动态将新的窗口设置应用到原生窗口对象上
        if self.window and self.window._window:
            try:
                win_cfg = config.get("window", {})
                self.window._window.on_top = win_cfg.get("always_on_top", True)
                self.window._window.resize(
                    win_cfg.get("width", 400),
                    win_cfg.get("height", 60)
                )
                self.bridge.toggle_click_through(win_cfg.get("click_through", False))
            except Exception as e:
                print(f"[Main] 动态更新窗口属性失败: {e}")

        self.window.push_config(config)


def main():
    """程序入口。"""
    app = App()
    try:
        app.start()
    except KeyboardInterrupt:
        app.stop()
    except Exception as e:
        print(f"[Main] 异常退出: {e}")
        import traceback
        traceback.print_exc()
        app.stop()


if __name__ == "__main__":
    main()
