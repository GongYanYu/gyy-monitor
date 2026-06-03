# 系统硬件监控悬浮窗 — 设计规格说明

**日期:** 2026-06-03
**状态:** 已确认
**技术栈:** Python 3 + pywebview (WebView2) + Vue 3 CDN

---

## 1. 项目概述

### 1.1 目标

一个 Windows 桌面悬浮窗工具，实时显示 CPU、GPU 的功耗、温度、占用等硬件信息。性能占用极小，所有显示内容、风格、行为均可通过 JSON 配置文件定制。

### 1.2 核心原则

- **轻量优先** — 内存占用目标 < 50MB，CPU 占用 < 1%
- **配置驱动** — 所有显示项、风格、布局、行为均由 config.json 控制
- **前端友好** — 悬浮窗 UI 使用 HTML/CSS/JS，前端工程师可直接定制
- **用户可控** — 托盘图标、右键菜单、热加载配置

### 1.3 目标用户

需要实时监控硬件状态的 Windows 用户（游戏玩家、开发者、超频爱好者）。

---

## 2. 架构设计

### 2.1 整体架构

```
┌──────────────────────────────────────────────────┐
│                  Python 主进程                     │
│                                                    │
│  ┌──────────────┐  ┌───────────────────────────┐  │
│  │  采集线程     │  │  pywebview 窗口            │  │
│  │  (1s 间隔)   │  │  ┌─────────────────────┐  │  │
│  │              │  │  │  WebView2 (Edge)     │  │  │
│  │  cpu.py      │  │  │  ┌─────────────────┐│  │  │
│  │  gpu.py      │──┼──│  │ Vue 3 App       ││  │  │
│  │  memory.py   │  │  │  │ - 数据渲染      ││  │  │
│  │  fps.py      │  │  │  │ - 主题切换      ││  │  │
│  │              │  │  │  │ - 右键菜单      ││  │  │
│  └──────────────┘  │  │  └─────────────────┘│  │  │
│                    │  └─────────────────────┘  │  │
│  ┌──────────────┐  └───────────────────────────┘  │
│  │ 配置管理      │                                  │
│  │ - 热加载      │  ┌───────────────────────────┐  │
│  │ - JSON 读写   │  │  系统托盘 (pystray)        │  │
│  └──────────────┘  │  - 显示/隐藏悬浮窗          │  │
│                    │  - 打开配置                 │  │
│                    │  - 退出                     │  │
│                    └───────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

### 2.2 目录结构

```
gyy-monitor/
├── main.py                 # 入口：加载配置、启动托盘、创建窗口
├── config.json              # 默认配置文件
├── collector/
│   ├── __init__.py
│   ├── base.py              # 采集器基类
│   ├── cpu.py               # CPU 数据（psutil）
│   ├── gpu.py               # GPU 数据（pynvml）
│   ├── memory.py            # 内存数据（psutil）
│   └── fps.py               # 帧率数据（DXGI 事件追踪）
├── ui/
│   ├── __init__.py
│   ├── window.py            # pywebview 窗口管理
│   ├── tray.py              # 系统托盘
│   └── bridge.py            # Python ↔ JS API 桥接
├── web/
│   ├── index.html           # 悬浮窗 HTML 入口
│   ├── style.css            # 内置主题样式
│   ├── app.js               # Vue 3 应用（CDN 加载）
│   └── themes/
│       ├── glass.css        # 暗色玻璃态预设
│       ├── minimal.css      # 极简数字风预设
│       └── hacker.css       # 终端黑客风预设
├── build.py                 # PyInstaller 打包脚本
└── requirements.txt         # Python 依赖
```

### 2.3 数据流

```
采集线程 (1s 定时器)
  │
  ├─ cpu.py    → cpu_usage, cpu_temp, cpu_power, cpu_fan
  ├─ gpu.py    → gpu_usage, gpu_temp, gpu_power, gpu_vram, gpu_fan
  ├─ memory.py → ram_usage, ram_used, ram_total
  └─ fps.py    → fps, fps_1pct_low
  │
  ▼
数据聚合为 dict → window.evaluate_js('app.updateMetrics({...})')
  │
  ▼
Vue 3 响应式数据更新 → 仅启用的指标卡片重新渲染
  │
  ▼
用户交互（右键菜单）→ pywebview.api.xxx() → Python 处理 → 保存配置
```

---

## 3. 核心组件设计

### 3.1 采集层（collector/）

#### base.py — 采集器基类

```python
class BaseCollector:
    """所有采集器继承此类"""

    def collect(self) -> dict:
        """返回指标字典，如 {'cpu_usage': 42.5, 'cpu_temp': 56}"""
        raise NotImplementedError

    @property
    def available_metrics(self) -> list[str]:
        """返回该采集器能提供的指标 ID 列表"""
        raise NotImplementedError
```

#### cpu.py — CPU 采集

- **数据源:** psutil (占用/频率)、Python 内置 WMI 或 psutil 补充（温度/功耗）
- **指标:** `cpu_usage`（整体%）、`cpu_temp`（℃ 封装温度）、`cpu_power`（W 封装功耗）、`cpu_fan`（RPM）
- **注意:** 温度和功耗在部分 AMD 桌面 CPU 上可能不可用，捕获异常返回 `None`

#### gpu.py — GPU 采集

- **数据源:** pynvml（NVIDIA 官方库）
- **指标:** `gpu_usage`（%）、`gpu_temp`（℃）、`gpu_power`（W）、`gpu_vram`（显存占用 %）、`gpu_fan`（RPM）
- **兼容性:** 对非 NVIDIA GPU（AMD/Intel），pynvml 不可用则标记 GPU 指标为 `unavailable`

#### memory.py — 内存采集

- **数据源:** psutil
- **指标:** `ram_usage`（%）、`ram_used`（GB）、`ram_total`（GB）

#### fps.py — 帧率采集

- **实现方式:** 通过 Windows ETW (Event Tracing for Windows) 监听 DXGI Present 事件
- **备选方案:** 若 ETW 权限不足，使用 PresentMon 命令行工具封装
- **指标:** `fps`（当前帧率）、`fps_1pct_low`（1% 最低帧）
- **注意:** 仅在用户启用 FPS 相关指标时才启动 ETW 监听，避免不必要的开销

### 3.2 UI 层（ui/）

#### window.py — 窗口管理

- 创建 pywebview 窗口，加载 `web/index.html`
- 设置无边框、置顶、透明背景等窗口属性
- 根据 config 控制窗口位置、大小、穿透模式
- 提供 `update_metrics(data)` 方法供采集线程调用

#### tray.py — 系统托盘

- 使用 pystray 创建托盘图标
- 菜单项：显示/隐藏悬浮窗、打开配置文件、切换风格（子菜单）、切换穿透模式、退出
- 左键点击托盘图标 → 切换悬浮窗显隐
- 退出时清理所有资源

#### bridge.py — JS Bridge API

暴露给前端调用的 Python 方法：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_config()` | — | `dict` | 获取完整配置 |
| `get_metrics()` | — | `dict` | 获取当前指标快照 |
| `set_config(key, value)` | `str, any` | `bool` | 修改单个配置并保存 |
| `set_config_bulk(patch)` | `dict` | `bool` | 批量修改配置并保存 |
| `restart_collector()` | — | `bool` | 重启采集线程 |
| `toggle_click_through(enabled)` | `bool` | `bool` | 切换穿透模式 |
| `open_config_file()` | — | `bool` | 用默认编辑器打开配置文件 |
| `get_available_metrics()` | — | `list` | 返回所有可用指标 ID |

### 3.3 前端（web/）

#### index.html

- 最小化 HTML 结构，通过 CDN 加载 Vue 3
- 不依赖构建工具，pywebview 直接加载本地文件
- `<div id="app">` 作为 Vue 挂载点

#### app.js — Vue 3 应用

```javascript
const { createApp, ref, computed, onMounted, onUnmounted } = Vue

const app = createApp({
  setup() {
    const config = ref({})       // 配置对象
    const metrics = ref({})      // 当前指标数据
    const enabledItems = computed(() => {
      // 从 config.metrics 中筛选 enabled=true 的项
      // 按 config.display.layout 排列
    })

    // 右键菜单逻辑
    // 主题 CSS 注入逻辑（custom_css）

    return { config, metrics, enabledItems }
  }
})
app.mount('#app')
```

**核心交互：**
- 页面加载 → 调用 `pywebview.api.get_config()` 获取配置
- 每收到 `updateMetrics(data)` → 更新 `metrics` ref → 自动重渲染
- 右键 → 弹出自定义菜单（风格切换、穿透模式、打开配置等）
- `custom_css` 非空时 → 动态创建 `<style>` 标签注入

#### 自定义 CSS 主题机制

配置文件中的 `custom_css` 字段支持两种模式：

1. **CSS 字符串** — 直接写在 config.json 中（适合小修改）
2. **文件路径** — 指向外部 .css 文件（适合完整主题）

```json
// 模式 1：直接注入
"custom_css": ".metric-card { font-weight: bold; } #app { background: red; }"

// 模式 2：引用外部文件
"custom_css": "C:/Users/me/my-theme.css"
```

前端检测 `custom_css` 以 `.css` 结尾则通过 bridge 读取文件内容，否则直接作为 CSS 注入。

#### 内置主题预设（web/themes/）

三套 CSS 文件，通过 `display.style` 配置切换：

- `minimal.css` — 极简数字风（默认），透明背景、彩色数字、无边框
- `glass.css` — 暗色玻璃态，半透明+模糊+圆角
- `hacker.css` — 终端黑客风，纯黑背景+等宽字体+霓虹绿

---

## 4. 配置文件规格

### 4.1 config.json 完整结构

```json
{
  "window": {
    "width": 400,
    "height": 60,
    "x": null,
    "y": null,
    "always_on_top": true,
    "click_through": false,
    "draggable": true,
    "opacity": 1.0,
    "frameless": true
  },
  "display": {
    "style": "minimal",
    "layout": "horizontal",
    "custom_css": null,
    "font_size": 14,
    "gap": 18,
    "padding": 8
  },
  "metrics": [
    {"id": "cpu_usage",  "enabled": true,  "label": "CPU",  "unit": "%",   "color": "#4fc3f7", "decimals": 0},
    {"id": "cpu_temp",   "enabled": true,  "label": "CPUT", "unit": "°C",  "color": "#4fc3f7", "decimals": 0},
    {"id": "cpu_power",  "enabled": false, "label": "CPUP", "unit": "W",   "color": "#4fc3f7", "decimals": 1},
    {"id": "cpu_fan",    "enabled": false, "label": "CPUF", "unit": "RPM", "color": "#4fc3f7", "decimals": 0},
    {"id": "gpu_usage",  "enabled": true,  "label": "GPU",  "unit": "%",   "color": "#81c784", "decimals": 0},
    {"id": "gpu_temp",   "enabled": true,  "label": "GPUT", "unit": "°C",  "color": "#81c784", "decimals": 0},
    {"id": "gpu_power",  "enabled": false, "label": "GPUP", "unit": "W",   "color": "#81c784", "decimals": 1},
    {"id": "gpu_vram",   "enabled": false, "label": "VRAM", "unit": "%",   "color": "#81c784", "decimals": 0},
    {"id": "gpu_fan",    "enabled": false, "label": "GPUF", "unit": "RPM", "color": "#81c784", "decimals": 0},
    {"id": "ram_usage",  "enabled": true,  "label": "RAM",  "unit": "%",   "color": "#ffb74d", "decimals": 0},
    {"id": "ram_used",   "enabled": false, "label": "RAMU", "unit": "GB",  "color": "#ffb74d", "decimals": 1},
    {"id": "fps",        "enabled": false, "label": "FPS",  "unit": "",    "color": "#ce93d8", "decimals": 0},
    {"id": "fps_1pct_low","enabled": false,"label": "1%L",  "unit": "",    "color": "#ce93d8", "decimals": 0}
  ],
  "update_interval_ms": 1000,
  "autostart": false
}
```

### 4.2 配置热加载

- 启动时读取 config.json
- 使用 `watchdog` 监听 config.json 文件变更
- 变更后 500ms 防抖，重新加载并推送到前端
- 若 JSON 格式错误，保留当前配置并在日志中警告

---

## 5. 技术选型与依赖

### 5.1 Python 依赖

| 包 | 版本 | 用途 |
|----|------|------|
| `pywebview` | >=5.0 | WebView2 窗口 |
| `psutil` | >=5.9 | CPU/内存数据 |
| `pynvml` | >=11.0 | NVIDIA GPU 数据 |
| `pystray` | >=0.19 | 系统托盘 |
| `Pillow` | >=10.0 | 托盘图标生成 |
| `watchdog` | >=4.0 | 配置文件热加载 |

### 5.2 前端依赖

| 库 | 加载方式 | 用途 |
|----|---------|------|
| Vue 3 | CDN (`unpkg.com/vue@3/dist/vue.global.prod.js`) | 响应式 UI |
| 无其他依赖 | — | 保持轻量 |

### 5.3 打包方案

- PyInstaller + `--windowed`（无控制台窗口）
- 打包后体积目标：< 50MB
- 使用 UPX 压缩可执行文件
- 资源文件（web/ 目录）作为 data 目录打包

---

## 6. 错误处理与兼容性

### 6.1 硬件数据不可用

- 所有采集器方法使用 try/except 包裹
- 不可用的指标值设为 `null`，前端显示 `--` 或 `N/A`
- GPU 采集器在非 NVIDIA 设备上优雅降级（标记全部 GPU 指标为 unavailable）

### 6.2 配置错误

- JSON 解析失败 → 使用内置默认配置，提示用户检查语法
- 配置值类型错误 → 使用该字段的默认值，记录警告

### 6.3 WebView2 缺失

- Windows 10 早期版本可能未安装 WebView2 Runtime
- 启动时检测，未安装则弹出提示并提供下载链接（微软官方 evergreen installer）

---

## 7. 后续扩展（不纳入首版）

- 传感器曲线图（小 sparkline）
- 多窗口支持（CPU 窗 + GPU 窗分离）
- 网络速率监控
- 磁盘 IO 监控
- 数据日志记录
- 过热/过载告警阈值

---

## 8. 自审清单

- [x] 无 TBD/TODO 占位符
- [x] 架构与功能描述一致
- [x] 范围聚焦，无功能蔓延
- [x] 所有需求可量化验证
- [x] 错误场景有明确处理策略
- [x] 兼容性边界已界定
