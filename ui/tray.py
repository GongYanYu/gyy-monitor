"""系统托盘模块。

使用 pystray 创建托盘图标和菜单。
"""

import threading
from PIL import Image, ImageDraw


class SystemTray:
    """管理系统托盘图标和菜单。"""

    def __init__(self, on_show=None, on_hide=None, on_toggle=None,
                 on_open_config=None, on_style_change=None, on_quit=None):
        self._icon = None
        self._callbacks = {
            "show": on_show or (lambda: None),
            "hide": on_hide or (lambda: None),
            "toggle": on_toggle or (lambda: None),
            "open_config": on_open_config or (lambda: None),
            "style_change": on_style_change or (lambda s: None),
            "quit": on_quit or (lambda: None),
        }

    def create(self):
        """创建托盘图标。"""
        import pystray

        image = self._create_icon_image()
        menu = self._build_menu()

        self._icon = pystray.Icon(
            name="gyy-monitor",
            title="gyy-monitor — 硬件监控",
            icon=image,
            menu=menu,
        )

    def run(self):
        """启动托盘（阻塞，应在独立线程中运行）。"""
        if self._icon:
            self._icon.run()

    def run_in_thread(self):
        """在 daemon 线程中运行托盘。"""
        thread = threading.Thread(target=self.run, daemon=True)
        thread.start()
        return thread

    def stop(self):
        """停止托盘。"""
        if self._icon:
            try:
                self._icon.stop()
            except Exception:
                pass

    def notify(self, title: str, message: str):
        """显示托盘通知。"""
        if self._icon and hasattr(self._icon, 'notify'):
            try:
                self._icon.notify(message, title=title)
            except Exception:
                pass

    # === 内部方法 ===

    def _build_menu(self):
        """构建托盘右键菜单。"""
        import pystray

        return pystray.Menu(
            pystray.MenuItem("显示/隐藏悬浮窗", self._on_toggle, default=True),
            pystray.Menu.SEPARATOR,
            pystray.MenuItem("编辑配置文件", self._on_open_config),
            pystray.Menu.SEPARATOR,
            pystray.MenuItem("风格", pystray.Menu(
                pystray.MenuItem("极简数字", lambda: self._on_style("minimal")),
                pystray.MenuItem("暗色玻璃", lambda: self._on_style("glass")),
                pystray.MenuItem("终端黑客", lambda: self._on_style("hacker")),
            )),
            pystray.Menu.SEPARATOR,
            pystray.MenuItem("退出", self._on_quit),
        )

    def _create_icon_image(self):
        """生成托盘图标（简洁的 M 字母图标）。"""
        size = 64
        image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        draw = ImageDraw.Draw(image)

        # 绘制圆角矩形背景
        draw.rounded_rectangle(
            [4, 4, size - 4, size - 4],
            radius=16,
            fill=(30, 30, 46, 255),
            outline=(137, 180, 250, 200),
            width=2,
        )

        # 绘制 "M" 字符（用简单的线条）
        draw.line([16, 44, 20, 18, 32, 34, 44, 18, 48, 44],
                  fill=(137, 180, 250, 255), width=4)

        return image

    def _on_toggle(self):
        self._callbacks["toggle"]()

    def _on_open_config(self):
        self._callbacks["open_config"]()

    def _on_style(self, style: str):
        self._callbacks["style_change"](style)

    def _on_quit(self):
        self._callbacks["quit"]()
