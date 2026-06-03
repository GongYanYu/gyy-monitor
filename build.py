"""PyInstaller 打包脚本。

将 gyy-monitor 打包为单个 Windows 可执行文件。
用法: python build.py
"""

import os
import subprocess
import sys
from pathlib import Path


def build():
    """执行 PyInstaller 打包。"""
    project_root = Path(__file__).parent

    # 确保 PyInstaller 已安装
    try:
        import PyInstaller
    except ImportError:
        print("正在安装 PyInstaller...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "pyinstaller"])

    # 构建命令
    cmd = [
        sys.executable, "-m", "PyInstaller",
        "--name=gyy-monitor",
        "--windowed",              # 无控制台窗口
        "--onefile",               # 单文件输出
        "--clean",
        "--add-data", f"web{os.pathsep}web",
        "--add-data", f"config.json{os.pathsep}.",
        "--hidden-import", "pystray._win32",
        "--hidden-import", "PIL._tkinter_finder",
        "--hidden-import", "pynvml",
        "--noconfirm",
        str(project_root / "main.py"),
    ]

    print(f"执行: {' '.join(cmd)}")
    subprocess.check_call(cmd)

    # 输出信息
    dist = project_root / "dist"
    exe = dist / "gyy-monitor.exe"
    if exe.exists():
        size_mb = exe.stat().st_size / (1024 * 1024)
        print(f"\n✅ 打包成功: {exe}")
        print(f"   文件大小: {size_mb:.1f} MB")
    else:
        print("\n❌ 打包失败，未找到输出文件")


if __name__ == "__main__":
    build()
