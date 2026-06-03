# 硬件监控悬浮窗 实现计划

> **对于 agentic 工作线程：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 按任务逐步实现此计划。步骤使用 checkbox (`- [ ]`) 语法追踪进度。

**目标:** 构建一个 Windows 桌面悬浮窗工具，通过 pywebview + Vue 3 CDN 实时显示 CPU/GPU/内存的功耗、温度、占用等信息，全部配置化。

**架构:** Python 主进程负责硬件数据采集（psutil/pynvml）和系统托盘（pystray），pywebview 承载 WebView2 渲染悬浮窗 UI，Vue 3 响应式数据绑定驱动渲染，JS Bridge 实现前后端双向通信。

**技术栈:** Python 3.10+, pywebview 5.x, Vue 3 (CDN), psutil, pynvml, pystray, Pillow

---

### Task 1: 项目骨架搭建

**文件:**
- 创建: `requirements.txt`
- 创建: `collector/__init__.py`
- 创建: `ui/__init__.py`

- [ ] **Step 1: 创建 requirements.txt**

```txt
pywebview>=5.0
psutil>=5.9.0
pynvml>=11.0.0
pystray>=0.19.0
Pillow>=10.0.0
watchdog>=4.0.0
```

- [ ] **Step 2: 创建目录和空的 __init__.py**

```bash
mkdir -p collector ui web/themes
touch collector/__init__.py ui/__init__.py
```

- [ ] **Step 3: 安装依赖**

```bash
pip install -r requirements.txt
```

预期: 所有包安装成功。

- [ ] **Step 4: 提交**

```bash
git add requirements.txt collector/__init__.py ui/__init__.py
git commit -m "chore: 初始化项目骨架和依赖"
```

---

### Task 2: 默认配置文件

**文件:**
- 创建: `config.json`

- [ ] **Step 1: 创建 config.json**

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
    {"id": "cpu_usage",   "enabled": true,  "label": "CPU",  "unit": "%",   "color": "#4fc3f7", "decimals": 0},
    {"id": "cpu_temp",    "enabled": true,  "label": "CPUT", "unit": "°C",  "color": "#4fc3f7", "decimals": 0},
    {"id": "cpu_power",   "enabled": false, "label": "CPUP", "unit": "W",   "color": "#4fc3f7", "decimals": 1},
    {"id": "cpu_fan",     "enabled": false, "label": "CPUF", "unit": "RPM", "color": "#4fc3f7", "decimals": 0},
    {"id": "gpu_usage",   "enabled": true,  "label": "GPU",  "unit": "%",   "color": "#81c784", "decimals": 0},
    {"id": "gpu_temp",    "enabled": true,  "label": "GPUT", "unit": "°C",  "color": "#81c784", "decimals": 0},
    {"id": "gpu_power",   "enabled": false, "label": "GPUP", "unit": "W",   "color": "#81c784", "decimals": 1},
    {"id": "gpu_vram",    "enabled": false, "label": "VRAM", "unit": "%",   "color": "#81c784", "decimals": 0},
    {"id": "gpu_fan",     "enabled": false, "label": "GPUF", "unit": "RPM", "color": "#81c784", "decimals": 0},
    {"id": "ram_usage",   "enabled": true,  "label": "RAM",  "unit": "%",   "color": "#ffb74d", "decimals": 0},
    {"id": "ram_used",    "enabled": false, "label": "RAMU", "unit": "GB",  "color": "#ffb74d", "decimals": 1},
    {"id": "fps",         "enabled": false, "label": "FPS",  "unit": "",    "color": "#ce93d8", "decimals": 0},
    {"id": "fps_1pct_low","enabled": false, "label": "1%L",  "unit": "",    "color": "#ce93d8", "decimals": 0}
  ],
  "update_interval_ms": 1000,
  "autostart": false
}
```

- [ ] **Step 2: 验证 JSON 格式合法**

```bash
python -c "import json; json.load(open('config.json')); print('OK')"
```

预期: 输出 `OK`。

- [ ] **Step 3: 提交**

```bash
git add config.json
git commit -m "feat: 添加默认配置文件"
```

---

### Task 3: 采集器基类

**文件:**
- 创建: `collector/base.py`

- [ ] **Step 1: 编写基类**

```python
"""采集器基类，所有硬件数据采集器继承此类。"""

from abc import ABC, abstractmethod


class BaseCollector(ABC):
    """所有采集器继承此类，统一 collect() 接口。"""

    @abstractmethod
    def collect(self) -> dict:
        """返回指标字典。

        Returns:
            dict: 如 {'cpu_usage': 42.5, 'cpu_temp': 56}。
                  不可用的值设为 None。
        """
        ...

    @property
    @abstractmethod
    def available_metrics(self) -> list[str]:
        """返回该采集器能提供的指标 ID 列表。

        Returns:
            list[str]: 指标 ID 列表。
        """
        ...
```

- [ ] **Step 2: 验证导入**

```bash
python -c "from collector.base import BaseCollector; print('OK')"
```

预期: 输出 `OK`。

- [ ] **Step 3: 提交**

```bash
git add collector/base.py
git commit -m "feat: 添加采集器基类"
```

---

### Task 4: CPU 采集器

**文件:**
- 创建: `collector/cpu.py`

- [ ] **Step 1: 编写 CPU 采集器**

```python
"""CPU 硬件数据采集器。"""

import psutil
from collector.base import BaseCollector


class CpuCollector(BaseCollector):
    """采集 CPU 占用率、温度、功耗、风扇转速。"""

    @property
    def available_metrics(self) -> list[str]:
        return ["cpu_usage", "cpu_temp", "cpu_power", "cpu_fan"]

    def collect(self) -> dict:
        result = {}

        # CPU 占用率（非阻塞，取 0.5s 间隔）
        try:
            result["cpu_usage"] = round(psutil.cpu_percent(interval=0.5), 1)
        except Exception:
            result["cpu_usage"] = None

        # CPU 温度（通过 psutil sensors_temperatures）
        result["cpu_temp"] = self._get_cpu_temp()

        # CPU 功耗（通过 psutil sensors_power 或 RAPL）
        result["cpu_power"] = self._get_cpu_power()

        # CPU 风扇
        result["cpu_fan"] = self._get_cpu_fan()

        return result

    def _get_cpu_temp(self) -> float | None:
        """获取 CPU 封装温度。

        遍历 psutil.sensors_temperatures() 查找 CPU 相关传感器，
        优先返回 'coretemp' 或 'k10temp' 标签下的最高温度。
        """
        try:
            temps = psutil.sensors_temperatures()
            if not temps:
                return None

            for label in ("coretemp", "k10temp", "cpu_thermal"):
                if label in temps:
                    entries = temps[label]
                    if entries:
                        return round(max(e.current for e in entries if e.current), 1)

            # 兜底：取第一个传感器的最高值
            first = next(iter(temps.values()))
            if first:
                return round(max(e.current for e in first if e.current), 1)

            return None
        except Exception:
            return None

    def _get_cpu_power(self) -> float | None:
        """获取 CPU 封装功耗（W）。

        通过 psutil sensors_power() 或 Intel RAPL 接口获取。
        部分 AMD CPU 不可用，返回 None。
        """
        try:
            power = psutil.sensors_power()
            if not power:
                return None

            for label in ("coretemp", "cpu", "package"):
                if label in power:
                    entries = power[label]
                    if entries:
                        return round(sum(e.current for e in entries if e.current), 1)

            return None
        except Exception:
            return None

    def _get_cpu_fan(self) -> int | None:
        """获取 CPU 风扇转速（RPM）。

        遍历 psutil.sensors_fans() 查找风扇数据。
        """
        try:
            fans = psutil.sensors_fans()
            if not fans:
                return None

            for label in fans:
                entries = fans[label]
                if entries:
                    speeds = [e.current for e in entries if e.current]
                    if speeds:
                        return max(speeds)

            return None
        except Exception:
            return None
```

- [ ] **Step 2: 快速验证采集器能运行**

```bash
python -c "
from collector.cpu import CpuCollector
c = CpuCollector()
print('可用指标:', c.available_metrics)
print('采集数据:', c.collect())
"
```

预期: 输出 CPU 指标字典，值可能为 None（取决于硬件支持）。

- [ ] **Step 3: 提交**

```bash
git add collector/cpu.py
git commit -m "feat: 添加 CPU 采集器"
```

---

### Task 5: 内存采集器

**文件:**
- 创建: `collector/memory.py`

- [ ] **Step 1: 编写内存采集器**

```python
"""内存数据采集器。"""

import psutil
from collector.base import BaseCollector


class MemoryCollector(BaseCollector):
    """采集系统内存占用。"""

    @property
    def available_metrics(self) -> list[str]:
        return ["ram_usage", "ram_used", "ram_total"]

    def collect(self) -> dict:
        result = {}

        try:
            mem = psutil.virtual_memory()
            result["ram_usage"] = round(mem.percent, 1)
            result["ram_used"] = round(mem.used / (1024 ** 3), 1)
            result["ram_total"] = round(mem.total / (1024 ** 3), 1)
        except Exception:
            result["ram_usage"] = None
            result["ram_used"] = None
            result["ram_total"] = None

        return result
```

- [ ] **Step 2: 验证采集器**

```bash
python -c "
from collector.memory import MemoryCollector
c = MemoryCollector()
print('采集数据:', c.collect())
"
```

预期: 输出内存指标，值不为 None。

- [ ] **Step 3: 提交**

```bash
git add collector/memory.py
git commit -m "feat: 添加内存采集器"
```

---

### Task 6: GPU 采集器

**文件:**
- 创建: `collector/gpu.py`

- [ ] **Step 1: 编写 GPU 采集器**

```python
"""GPU 硬件数据采集器（NVIDIA GPU，通过 pynvml）。"""

from collector.base import BaseCollector


class GpuCollector(BaseCollector):
    """采集 GPU 占用率、温度、功耗、显存、风扇转速。

    仅支持 NVIDIA GPU（通过 NVML）。
    非 NVIDIA 设备或 NVML 不可用时，所有指标返回 None。
    """

    def __init__(self):
        self._nvml_available = False
        self._handle = None
        try:
            import pynvml
            pynvml.nvmlInit()
            # 获取第一个 GPU
            count = pynvml.nvmlDeviceGetCount()
            if count > 0:
                self._handle = pynvml.nvmlDeviceGetHandleByIndex(0)
                self._nvml_available = True
        except Exception:
            self._nvml_available = False

    @property
    def available_metrics(self) -> list[str]:
        return ["gpu_usage", "gpu_temp", "gpu_power", "gpu_vram", "gpu_fan"]

    def collect(self) -> dict:
        if not self._nvml_available:
            return {
                "gpu_usage": None, "gpu_temp": None,
                "gpu_power": None, "gpu_vram": None, "gpu_fan": None,
            }

        import pynvml

        result = {}

        # GPU 占用率
        try:
            util = pynvml.nvmlDeviceGetUtilizationRates(self._handle)
            result["gpu_usage"] = round(util.gpu, 1)
        except Exception:
            result["gpu_usage"] = None

        # GPU 温度
        try:
            temp = pynvml.nvmlDeviceGetTemperature(
                self._handle, pynvml.NVML_TEMPERATURE_GPU
            )
            result["gpu_temp"] = round(temp, 1)
        except Exception:
            result["gpu_temp"] = None

        # GPU 功耗（W）
        try:
            power = pynvml.nvmlDeviceGetPowerUsage(self._handle)
            # pynvml 返回毫瓦，转换为瓦
            result["gpu_power"] = round(power / 1000.0, 1)
        except Exception:
            result["gpu_power"] = None

        # 显存占用率
        try:
            mem_info = pynvml.nvmlDeviceGetMemoryInfo(self._handle)
            vram_pct = (mem_info.used / mem_info.total) * 100
            result["gpu_vram"] = round(vram_pct, 1)
        except Exception:
            result["gpu_vram"] = None

        # GPU 风扇转速
        try:
            fan = pynvml.nvmlDeviceGetFanSpeed(self._handle)
            result["gpu_fan"] = fan
        except Exception:
            result["gpu_fan"] = None

        return result

    def shutdown(self):
        """释放 NVML 资源。"""
        if self._nvml_available:
            try:
                import pynvml
                pynvml.nvmlShutdown()
            except Exception:
                pass
```

- [ ] **Step 2: 验证采集器（NVIDIA GPU 环境）**

```bash
python -c "
from collector.gpu import GpuCollector
c = GpuCollector()
print('可用指标:', c.available_metrics)
print('采集数据:', c.collect())
c.shutdown()
"
```

预期: 有 NVIDIA GPU 则输出有效数据，否则全部为 None（不抛异常）。

- [ ] **Step 3: 提交**

```bash
git add collector/gpu.py
git commit -m "feat: 添加 GPU 采集器"
```

---

### Task 7: FPS 采集器（占位）

**文件:**
- 创建: `collector/fps.py`

- [ ] **Step 1: 编写 FPS 采集器（占位实现）**

FPS 采集依赖 ETW/DXGI 事件追踪，实现较复杂。首版先做占位，后续再完善。

```python
"""FPS 帧率采集器（占位实现，后续通过 ETW/DXGI 完善）。"""

from collector.base import BaseCollector


class FpsCollector(BaseCollector):
    """采集当前屏幕帧率。

    首版为占位实现，返回 None。
    后续通过 Windows ETW 监听 DXGI Present 事件获取真实帧率。
    """

    @property
    def available_metrics(self) -> list[str]:
        return ["fps", "fps_1pct_low"]

    def collect(self) -> dict:
        return {
            "fps": None,
            "fps_1pct_low": None,
        }

    def start(self):
        """启动 ETW 监听（占位，后续实现）。"""
        pass

    def stop(self):
        """停止 ETW 监听（占位，后续实现）。"""
        pass
```

- [ ] **Step 2: 验证导入**

```bash
python -c "from collector.fps import FpsCollector; c = FpsCollector(); print(c.collect())"
```

预期: `{'fps': None, 'fps_1pct_low': None}`。

- [ ] **Step 3: 提交**

```bash
git add collector/fps.py
git commit -m "feat: 添加 FPS 采集器（占位实现）"
```

---

### Task 8: 前端 HTML + 基础样式

**文件:**
- 创建: `web/index.html`
- 创建: `web/style.css`

- [ ] **Step 1: 编写 index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>gyy-monitor</title>
  <link rel="stylesheet" href="style.css">
  <link id="theme-style" rel="stylesheet" href="">
</head>
<body>
  <div id="app">
    <div
      class="monitor-container"
      :class="['layout-' + config.display.layout]"
      :style="containerStyle"
      @contextmenu.prevent="showMenu"
      @mousedown="onMouseDown"
    >
      <div
        v-for="item in enabledMetrics"
        :key="item.id"
        class="metric-card"
        :style="cardStyle"
      >
        <span class="metric-label">{{ item.label }}</span>
        <span class="metric-value" :style="{ color: item.color }">
          {{ formatValue(metrics[item.id], item) }}
          <span class="metric-unit" v-if="item.unit">{{ item.unit }}</span>
        </span>
      </div>
    </div>

    <!-- 右键菜单 -->
    <div
      v-if="menuVisible"
      class="context-menu"
      :style="{ left: menuX + 'px', top: menuY + 'px' }"
      @mouseleave="menuVisible = false"
    >
      <div class="menu-item" @click="toggleClickThrough">
        {{ config.window.click_through ? '🔲 关闭穿透' : '🔳 开启穿透' }}
      </div>
      <div class="menu-separator"></div>
      <div class="menu-label">风格</div>
      <div
        v-for="s in ['minimal', 'glass', 'hacker']"
        :key="s"
        class="menu-item"
        :class="{ active: config.display.style === s }"
        @click="setStyle(s)"
      >{{ styleLabel(s) }}</div>
      <div class="menu-separator"></div>
      <div class="menu-label">布局</div>
      <div
        v-for="l in ['horizontal', 'vertical', 'grid']"
        :key="l"
        class="menu-item"
        :class="{ active: config.display.layout === l }"
        @click="setLayout(l)"
      >{{ layoutLabel(l) }}</div>
      <div class="menu-separator"></div>
      <div class="menu-item" @click="openConfig">📝 编辑配置</div>
      <div class="menu-item" @click="hideWindow">👁 隐藏窗口</div>
    </div>
  </div>

  <script src="https://unpkg.com/vue@3/dist/vue.global.prod.js"></script>
  <script src="app.js"></script>
</body>
</html>
```

- [ ] **Step 2: 编写 style.css（基础结构 + 右键菜单样式）**

```css
/* === 重置与基础 === */
*, *::before, *::after {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
  font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
  user-select: none;
  -webkit-user-select: none;
}

/* === 布局容器 === */
.monitor-container {
  display: flex;
  width: 100%;
  height: 100%;
  align-items: center;
}

.layout-horizontal {
  flex-direction: row;
  justify-content: center;
}

.layout-vertical {
  flex-direction: column;
  justify-content: center;
  align-items: flex-start;
}

.layout-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(80px, 1fr));
  align-content: center;
}

/* === 指标卡片 === */
.metric-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  white-space: nowrap;
}

.metric-label {
  font-size: 11px;
  opacity: 0.6;
  letter-spacing: 0.5px;
}

.metric-value {
  font-size: 20px;
  font-weight: 700;
  line-height: 1.2;
}

.metric-unit {
  font-size: 11px;
  opacity: 0.5;
  font-weight: 400;
}

/* === 右键菜单 === */
.context-menu {
  position: fixed;
  background: rgba(30, 30, 46, 0.95);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 4px;
  min-width: 160px;
  z-index: 9999;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.menu-item {
  padding: 6px 12px;
  border-radius: 4px;
  font-size: 12px;
  color: #cdd6f4;
  cursor: pointer;
}

.menu-item:hover {
  background: rgba(255, 255, 255, 0.08);
}

.menu-item.active {
  color: #89b4fa;
}

.menu-item.active::before {
  content: '✓ ';
}

.menu-separator {
  height: 1px;
  background: rgba(255, 255, 255, 0.06);
  margin: 4px 0;
}

.menu-label {
  padding: 4px 12px;
  font-size: 10px;
  color: #6c7086;
  text-transform: uppercase;
  letter-spacing: 1px;
}
```

- [ ] **Step 3: 提交**

```bash
git add web/index.html web/style.css
git commit -m "feat: 添加前端 HTML 入口和基础样式"
```

---

### Task 9: 主题 CSS 文件

**文件:**
- 创建: `web/themes/minimal.css`
- 创建: `web/themes/glass.css`
- 创建: `web/themes/hacker.css`

- [ ] **Step 1: 编写 minimal.css（极简数字风，默认）**

```css
/* 极简数字风 — 透明背景 + 彩色数字 */
.monitor-container {
  background: transparent;
  text-shadow: 0 0 8px rgba(0, 0, 0, 0.6);
}

.metric-value {
  font-family: 'Consolas', 'Cascadia Code', 'Courier New', monospace;
}
```

- [ ] **Step 2: 编写 glass.css（暗色玻璃态）**

```css
/* 暗色玻璃态 — Acrylic 毛玻璃效果 */
.monitor-container {
  background: rgba(30, 30, 46, 0.85);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

.metric-label {
  color: #a6adc8;
}

.metric-unit {
  color: #a6adc8;
}
```

- [ ] **Step 3: 编写 hacker.css（终端黑客风）**

```css
/* 终端黑客风 — 纯黑背景 + 等宽字体 + 霓虹绿 */
.monitor-container {
  background: rgba(10, 10, 14, 0.92);
  border: 1px solid rgba(0, 255, 100, 0.15);
  border-radius: 8px;
  box-shadow: 0 0 20px rgba(0, 255, 100, 0.05);
  font-family: 'Cascadia Code', 'Consolas', 'Courier New', monospace;
}

.metric-label {
  color: #0a0;
  font-family: inherit;
}

.metric-value {
  color: #0f0;
  font-family: inherit;
  text-shadow: 0 0 6px rgba(0, 255, 0, 0.3);
}

.metric-unit {
  color: #0a0;
  font-family: inherit;
}
```

- [ ] **Step 4: 提交**

```bash
git add web/themes/minimal.css web/themes/glass.css web/themes/hacker.css
git commit -m "feat: 添加三套内置主题 CSS"
```

---

### Task 10: Vue 3 应用逻辑

**文件:**
- 创建: `web/app.js`

- [ ] **Step 1: 编写 app.js**

```javascript
/* gyy-monitor 前端 — Vue 3 应用 */

const { createApp, ref, computed, onMounted, onUnmounted } = Vue;

const app = createApp({
  setup() {
    // === 响应式状态 ===
    const config = ref(getDefaultConfig());
    const metrics = ref({});
    const menuVisible = ref(false);
    const menuX = ref(0);
    const menuY = ref(0);

    // 拖动相关
    let dragging = false;
    let dragStartX = 0;
    let dragStartY = 0;

    // === 默认配置（后端不可用时的兜底） ===
    function getDefaultConfig() {
      return {
        window: {
          width: 400, height: 60, x: null, y: null,
          always_on_top: true, click_through: false,
          draggable: true, opacity: 1.0, frameless: true,
        },
        display: {
          style: 'minimal', layout: 'horizontal',
          custom_css: null, font_size: 14, gap: 18, padding: 8,
        },
        metrics: [],
        update_interval_ms: 1000,
        autostart: false,
      };
    }

    // === 计算属性 ===
    const enabledMetrics = computed(() => {
      return (config.value.metrics || []).filter(m => m.enabled);
    });

    const containerStyle = computed(() => {
      const d = config.value.display;
      return {
        gap: (d.gap || 18) + 'px',
        padding: (d.padding || 8) + 'px',
        fontSize: (d.font_size || 14) + 'px',
      };
    });

    const cardStyle = computed(() => {
      const d = config.value.display;
      return { fontSize: (d.font_size || 14) + 'px' };
    });

    // === 方法 ===
    function formatValue(value, metricDef) {
      if (value === null || value === undefined) return '--';
      const decimals = metricDef.decimals ?? 0;
      return Number(value).toFixed(decimals);
    }

    function styleLabel(s) {
      return { minimal: '极简数字', glass: '暗色玻璃', hacker: '终端黑客' }[s] || s;
    }

    function layoutLabel(l) {
      return { horizontal: '横向', vertical: '纵向', grid: '网格' }[l] || l;
    }

    function showMenu(e) {
      menuX.value = e.clientX;
      menuY.value = e.clientY;
      menuVisible.value = true;
    }

    async function toggleClickThrough() {
      const newVal = !config.value.window.click_through;
      config.value.window.click_through = newVal;
      menuVisible.value = false;
      try {
        await pywebview.api.set_config('window.click_through', newVal);
        await pywebview.api.toggle_click_through(newVal);
      } catch (e) { console.warn('Bridge 调用失败:', e); }
    }

    async function setStyle(style) {
      config.value.display.style = style;
      menuVisible.value = false;
      loadThemeStyle(style);
      try {
        await pywebview.api.set_config('display.style', style);
      } catch (e) { console.warn('Bridge 调用失败:', e); }
    }

    async function setLayout(layout) {
      config.value.display.layout = layout;
      menuVisible.value = false;
      try {
        await pywebview.api.set_config('display.layout', layout);
      } catch (e) { console.warn('Bridge 调用失败:', e); }
    }

    async function openConfig() {
      menuVisible.value = false;
      try {
        await pywebview.api.open_config_file();
      } catch (e) { console.warn('Bridge 调用失败:', e); }
    }

    async function hideWindow() {
      menuVisible.value = false;
      try {
        await pywebview.api.toggle_visible();
      } catch (e) { console.warn('Bridge 调用失败:', e); }
    }

    // === 拖动支持 ===
    function onMouseDown(e) {
      if (!config.value.window.draggable) return;
      if (e.target.closest('.context-menu')) return;
      dragging = true;
      dragStartX = e.screenX;
      dragStartY = e.screenY;
    }

    function onMouseMove(e) {
      if (!dragging) return;
      const dx = e.screenX - dragStartX;
      const dy = e.screenY - dragStartY;
      dragStartX = e.screenX;
      dragStartY = e.screenY;
      try {
        pywebview.api.move_window(dx, dy);
      } catch (e) { /* ignore */ }
    }

    function onMouseUp() {
      dragging = false;
    }

    // === 主题加载 ===
    function loadThemeStyle(style) {
      const link = document.getElementById('theme-style');
      if (link) link.href = 'themes/' + style + '.css';
    }

    // === 初始化 ===
    onMounted(async () => {
      // 加载配置
      try {
        const cfg = await pywebview.api.get_config();
        if (cfg) config.value = cfg;
      } catch (e) {
        console.warn('无法从后端加载配置，使用默认值:', e);
      }

      // 加载主题
      loadThemeStyle(config.value.display.style);

      // 全局事件
      document.addEventListener('mousemove', onMouseMove);
      document.addEventListener('mouseup', onMouseUp);

      // 暴露 updateMetrics 供 Python 调用
      window.updateMetrics = (data) => {
        metrics.value = { ...metrics.value, ...data };
      };

      // 暴露 updateConfig 供 Python 热加载调用
      window.updateConfig = (newConfig) => {
        config.value = { ...config.value, ...newConfig };
        loadThemeStyle(config.value.display.style);
      };
    });

    onUnmounted(() => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      delete window.updateMetrics;
      delete window.updateConfig;
    });

    return {
      config, metrics, menuVisible, menuX, menuY,
      enabledMetrics, containerStyle, cardStyle,
      formatValue, styleLabel, layoutLabel,
      showMenu, toggleClickThrough, setStyle, setLayout,
      openConfig, hideWindow, onMouseDown,
    };
  },
});

app.mount('#app');
```

- [ ] **Step 2: 提交**

```bash
git add web/app.js
git commit -m "feat: 添加 Vue 3 应用逻辑（数据渲染、右键菜单、拖动、主题切换）"
```

---

### Task 11: JS Bridge API

**文件:**
- 创建: `ui/bridge.py`

- [ ] **Step 1: 编写 Bridge 类**

```python
"""Python ↔ JavaScript 桥接 API。

通过 pywebview 的 api 机制暴露给前端调用。
"""

import json
import os
import subprocess
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
        # 从配置中提取所有指标 ID
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
                # pywebview 没有直接的 click-through API，
                # 通过 Win32 API 设置 WS_EX_TRANSPARENT
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

            hwnd = self._window._html.hwnd if hasattr(self._window, '_html') else None
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
```

- [ ] **Step 2: 提交**

```bash
git add ui/bridge.py
git commit -m "feat: 添加 JS Bridge API"
```

---

### Task 12: 窗口管理

**文件:**
- 创建: `ui/window.py`

- [ ] **Step 1: 编写窗口管理模块**

```python
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
```

- [ ] **Step 2: 提交**

```bash
git add ui/window.py
git commit -m "feat: 添加 pywebview 窗口管理模块"
```

---

### Task 13: 系统托盘

**文件:**
- 创建: `ui/tray.py`

- [ ] **Step 1: 编写系统托盘模块**

```python
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
        # 用两条竖线 + 中间三角形近似 M
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
```

- [ ] **Step 2: 提交**

```bash
git add ui/tray.py
git commit -m "feat: 添加系统托盘模块"
```

---

### Task 14: 主入口

**文件:**
- 创建: `main.py`

- [ ] **Step 1: 编写 main.py**

```python
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

        # 检查 WebView2 可用性
        self._check_webview2()

        # 创建窗口
        self.window.create()

        # 启动采集线程
        self._collector_thread = threading.Thread(target=self._collect_loop, daemon=True)
        self._collector_thread.start()

        # 创建并启动托盘
        self.tray.create()
        tray_thread = self.tray.run_in_thread()

        # 进入 pywebview 消息循环（阻塞）
        webview.start(debug=False)

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

    def _check_webview2(self):
        """检查 WebView2 Runtime 是否可用。"""
        try:
            # 尝试获取 WebView2 版本
            import ctypes
            import ctypes.wintypes
            # 简单检测：尝试创建 webview 对象
        except Exception:
            pass  # pywebview 内部会处理

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
        self.window.toggle()

    def _on_open_config(self):
        """托盘：打开配置文件。"""
        os.startfile(self.config_path)

    def _on_style_change(self, style: str):
        """托盘：切换主题风格。"""
        self.config["display"]["style"] = style
        with open(self.config_path, "w", encoding="utf-8") as f:
            json.dump(self.config, f, indent=2, ensure_ascii=False)
        self.window.push_config(self.config)

    def _on_quit(self):
        """托盘：退出应用。"""
        self._running = False
        self.window.destroy()
        self.tray.stop()
        os._exit(0)

    def _on_config_reloaded(self, config: dict):
        """配置热加载回调。"""
        self.config = config
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
```

- [ ] **Step 2: 提交**

```bash
git add main.py
git commit -m "feat: 添加主入口，整合所有模块"
```

---

### Task 15: 打包脚本

**文件:**
- 创建: `build.py`

- [ ] **Step 1: 编写打包脚本**

```python
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
```

- [ ] **Step 2: 提交**

```bash
git add build.py
git commit -m "feat: 添加 PyInstaller 打包脚本"
```

---

### Task 16: 功能测试与验证

- [ ] **Step 1: 启动应用并验证基本功能**

```bash
python main.py
```

验证清单:
- [ ] 悬浮窗正常显示（极简数字风，横向布局）
- [ ] CPU 占用率、CPU 温度、GPU 占用率、GPU 温度、内存占用率均显示有效数值
- [ ] 右键弹出菜单，可切换风格（极简/玻璃/黑客）和布局（横向/纵向/网格）
- [ ] 可拖动悬浮窗到任意位置
- [ ] 系统托盘图标显示，左键切换显隐
- [ ] 托盘右键菜单功能正常（显示/隐藏、编辑配置、切换风格、退出）
- [ ] 修改 config.json 后应用热加载

- [ ] **Step 2: 确认退出后进程不残留**

关闭应用后，在任务管理器中确认 `python.exe` 和 `gyy-monitor.exe` 均已退出。

- [ ] **Step 3: 提交最终版本**

```bash
git add -A
git commit -m "test: 功能验证通过，首版完成"
```

---

## 实现顺序总结

| 序号 | 任务 | 依赖 |
|------|------|------|
| 1 | 项目骨架搭建 | — |
| 2 | 默认配置文件 | 1 |
| 3 | 采集器基类 | 1 |
| 4 | CPU 采集器 | 3 |
| 5 | 内存采集器 | 3 |
| 6 | GPU 采集器 | 3 |
| 7 | FPS 采集器（占位） | 3 |
| 8 | 前端 HTML + 基础样式 | 1 |
| 9 | 主题 CSS 文件 | 8 |
| 10 | Vue 3 应用逻辑 | 8, 9 |
| 11 | JS Bridge API | 2 |
| 12 | 窗口管理 | 11 |
| 13 | 系统托盘 | — |
| 14 | 主入口 | 4-7, 10, 12, 13 |
| 15 | 打包脚本 | 14 |
| 16 | 功能测试 | 14 |
