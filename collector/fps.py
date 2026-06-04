"""FPS 帧率采集器（使用 Windows ETW 捕获 DXGI Present 事件）。"""

import sys
import time
import threading
import ctypes
from collections import defaultdict
from collector.base import BaseCollector

# 固定的 ETW 会话名称，确保每次启动时能清理上次残留的会话
_ETW_SESSION_NAME = "GyyMonitor-DXGI-FPS"


class FpsCollector(BaseCollector):
    """通过 ETW 监听 Microsoft-Windows-DXGI 的 Present 提交事件，获取当前前台活跃窗口的真实 FPS。"""

    def __init__(self):
        self._fps = 0.0
        self._fps_1pct_low = 0.0
        self._running = False
        self._session = None

        # 用于存储各个进程和 swapchain 的帧时间戳
        # 键为 (pid, p_swap_chain)
        self._frame_timestamps = defaultdict(list)
        self._last_event_time = {}       # 键为 (pid, p_swap_chain)，值为 time.perf_counter()
        self._pid_last_event_time = {}   # 键为 pid，值为 time.perf_counter()
        self._lock = threading.Lock()

        # 初始化时自动启动监听服务
        self.start()

    @property
    def available_metrics(self) -> list[str]:
        return ["fps", "fps_1pct_low"]

    def _cleanup_stale_session(self):
        """清理上次程序崩溃/强杀后残留在 Windows 内核中的 ETW 会话。"""
        # 1. 清理固定名称的会话
        try:
            import etw
            from etw.etw import TraceProperties
            import etw.evntrace as et

            props = TraceProperties()
            et.ControlTraceW(
                et.TRACEHANDLE(0),
                _ETW_SESSION_NAME,
                props.get(),
                et.EVENT_TRACE_CONTROL_STOP,
            )
            print(f"[FPS] 成功清理了上次残留的 ETW 会话 '{_ETW_SESSION_NAME}'")
        except Exception:
            pass

        # 2. 清理之前版本遗留的 UUID 命名的孤儿会话（由 pywintrace 默认随机生成）
        try:
            import subprocess
            import re
            result = subprocess.run(
                ["logman", "query", "-ets"],
                capture_output=True, text=True, encoding="gbk", errors="replace",
                timeout=5,
            )
            uuid_pattern = re.compile(r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', re.IGNORECASE)
            for line in result.stdout.splitlines():
                name = line.strip().split()[0] if line.strip() else ""
                if uuid_pattern.match(name):
                    try:
                        subprocess.run(
                            ["logman", "stop", name, "-ets"],
                            capture_output=True, timeout=3,
                        )
                        print(f"[FPS] 清理了孤儿 ETW 会话: {name}")
                    except Exception:
                        pass
        except Exception:
            pass

    def start(self):
        """启动 ETW 监听会话。"""
        if sys.platform != "win32":
            return

        # 检查是否为管理员权限，ETW 需要管理员权限
        try:
            is_admin = ctypes.windll.shell32.IsUserAnAdmin()
        except Exception:
            is_admin = False

        if not is_admin:
            print("[FPS] 无管理员权限，无法启动 ETW 监听会话，请以管理员身份运行软件。")
            return

        try:
            import etw

            # 1. 先清理上次可能残留的同名会话
            self._cleanup_stale_session()

            # 2. 创建新的 ETW 会话
            dxgi_guid = "{ca11c036-0102-4a2d-a6ad-f03cfed5d3c9}"
            provider = etw.ProviderInfo('Microsoft-Windows-DXGI', etw.GUID(dxgi_guid))

            # 使用固定会话名称，不再使用随机 UUID
            # 不使用 event_id_filters 参数，改在回调中手动过滤（更可靠）
            self._session = etw.ETW(
                session_name=_ETW_SESSION_NAME,
                providers=[provider],
                event_callback=self._event_callback,
            )
            self._session.start()
            self._running = True
            print("[FPS] 成功在后台开启 ETW DXGI 帧率监听会话！")
        except Exception as e:
            print(f"[FPS] 启动 ETW 监听失败: {e}")
            import traceback
            traceback.print_exc()
            self._session = None

    def _event_callback(self, event):
        """ETW 事件回调函数。"""
        try:
            event_id, event_data = event
            # 只处理 Event ID 42 (DXGI Present Start)
            if event_id != 42:
                return

            header = event_data.get("EventHeader", {})
            pid = header.get("ProcessId")
            if not pid:
                return

            # pSwapChain 用于区分同一进程中的不同交换链（游戏主画面 vs 覆盖层）
            # 如果字段缺失则降级为 0（仍然可以正常统计）
            p_swap_chain = event_data.get("pSwapChain", 0)

            # 使用 ETW 原生 TimeStamp（100ns 精度），避免 Python 批量回调导致的时间挤压
            event_time = header.get("TimeStamp")
            if event_time is not None:
                now = event_time / 10000000.0  # 100ns → 秒
            else:
                now = time.perf_counter()

            now_perf = time.perf_counter()
            key = (pid, p_swap_chain)

            with self._lock:
                timestamps = self._frame_timestamps[key]
                timestamps.append(now)

                # 记录该 swapchain 和 pid 的最新活跃时间（使用 perf_counter 用于超时判断）
                self._last_event_time[key] = now_perf
                self._pid_last_event_time[pid] = now_perf

                # 只保留最近 3 秒内的时间戳
                cutoff = now - 3.0
                while timestamps and timestamps[0] < cutoff:
                    timestamps.pop(0)
        except Exception:
            pass

    def _get_active_pid(self) -> int:
        """获取当前前台活跃窗口的 Process ID。"""
        try:
            user32 = ctypes.windll.user32
            hwnd = user32.GetForegroundWindow()
            if not hwnd:
                return 0
            pid = ctypes.c_ulong()
            user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
            return pid.value
        except Exception:
            return 0

    def collect(self) -> dict:
        """计算并获取当前活跃进程的 FPS 和 1% Low FPS。"""
        if sys.platform != "win32" or not self._running:
            return {"fps": None, "fps_1pct_low": None}

        active_pid = self._get_active_pid()
        if active_pid <= 0:
            return {"fps": 0.0, "fps_1pct_low": 0.0}

        now_perf = time.perf_counter()

        # 检查该 PID 最近 1.5 秒内是否有任何 Present 事件
        if now_perf - self._pid_last_event_time.get(active_pid, 0.0) > 1.5:
            return {"fps": 0.0, "fps_1pct_low": 0.0}

        with self._lock:
            # 找出属于 active_pid 且最近 1.5 秒内有活动的所有 swapchain
            active_keys = [k for k in self._frame_timestamps.keys() if k[0] == active_pid]

            swapchain_results = []
            for key in active_keys:
                if now_perf - self._last_event_time.get(key, 0.0) > 1.5:
                    continue

                timestamps = list(self._frame_timestamps[key])
                if len(timestamps) < 2:
                    continue

                # 过滤出最近 1.5 秒内的时间戳
                latest_ts = timestamps[-1]
                cutoff_ts = latest_ts - 1.5
                recent_ts = [t for t in timestamps if t >= cutoff_ts]

                if len(recent_ts) < 2:
                    continue

                total_frames = len(recent_ts)
                time_span = recent_ts[-1] - recent_ts[0]
                if time_span > 0:
                    fps = (total_frames - 1) / time_span
                else:
                    fps = 0.0

                # 计算 1% Low FPS
                frame_times = []
                for i in range(1, len(recent_ts)):
                    frame_times.append(recent_ts[i] - recent_ts[i - 1])

                if len(frame_times) >= 10:
                    frame_times.sort()
                    num_1pct = max(1, int(len(frame_times) * 0.01))
                    slowest = frame_times[-num_1pct:]
                    avg_slowest = sum(slowest) / len(slowest)
                    fps_1pct_low = 1.0 / avg_slowest if avg_slowest > 0 else 0.0
                else:
                    fps_1pct_low = fps

                swapchain_results.append({"fps": fps, "fps_1pct_low": fps_1pct_low})

        if not swapchain_results:
            return {"fps": 0.0, "fps_1pct_low": 0.0}

        # 选取 FPS 最高的那个 swapchain（即主游戏渲染窗口）
        best = max(swapchain_results, key=lambda x: x["fps"])

        return {
            "fps": round(best["fps"], 0),
            "fps_1pct_low": round(best["fps_1pct_low"], 0),
        }

    def stop(self):
        """停止 ETW 监听。"""
        if self._session:
            try:
                self._session.stop()
                print("[FPS] 成功停止并释放 ETW 帧率监听会话。")
            except Exception:
                pass
            finally:
                self._session = None
                self._running = False

    def __del__(self):
        self.stop()
