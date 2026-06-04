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
        metrics: [
          {"id": "cpu_usage", "enabled": true, "label": "CPU", "unit": "%", "color": "#4fc3f7", "decimals": 0},
          {"id": "cpu_temp", "enabled": true, "label": "CPUT", "unit": "°C", "color": "#4fc3f7", "decimals": 0},
          {"id": "gpu_usage", "enabled": true, "label": "GPU", "unit": "%", "color": "#81c784", "decimals": 0},
          {"id": "gpu_temp", "enabled": true, "label": "GPUT", "unit": "°C", "color": "#81c784", "decimals": 0},
          {"id": "ram_usage", "enabled": true, "label": "RAM", "unit": "%", "color": "#ffb74d", "decimals": 0},
        ],
        update_interval_ms: 1000,
        autostart: false,
        fps_only_in_game: true,
      };
    }

    // === 计算属性 ===
    const enabledMetrics = computed(() => {
      const fpsIds = ['fps', 'fps_1pct_low'];
      return (config.value.metrics || []).filter(m => {
        if (!m.enabled) return false;
        // 当 fps_only_in_game 开启时，如果 FPS/1%L 值为 0 或无数据，则隐藏该指标
        if (config.value.fps_only_in_game && fpsIds.includes(m.id)) {
          const val = metrics.value[m.id];
          if (val === null || val === undefined || val === 0 || val === 0.0) return false;
        }
        return true;
      });
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
      const num = Number(value);
      if (isNaN(num)) return '--';
      const decimals = metricDef.decimals ?? 0;
      return num.toFixed(decimals);
    }

    function styleLabel(s) {
      return { minimal: '极简数字', glass: '暗色玻璃', hacker: '终端黑客' }[s] || s;
    }

    function layoutLabel(l) {
      return { horizontal: '横向', vertical: '纵向', grid: '网格' }[l] || l;
    }

    function showMenu(e) {
      try {
        if (window.pywebview && window.pywebview.api && window.pywebview.api.show_context_menu) {
          pywebview.api.show_context_menu();
          return;
        }
      } catch (err) {
        console.warn('调用原生右键菜单失败:', err);
      }
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

    // === 自动调整窗口大小以完全显示所有指标数据 ===
    async function adjustWindowSize(allowShrink = true) {
      await Vue.nextTick();
      const container = document.querySelector('.monitor-container');
      if (!container) return;

      const cards = container.querySelectorAll('.metric-card');
      if (cards.length === 0) return;

      const layout = config.value.display.layout;
      const padding = config.value.display.padding || 8;
      const gap = config.value.display.gap || 18;

      let reqWidth = 0;
      let reqHeight = 0;

      if (layout === 'horizontal') {
        let totalCardsWidth = 0;
        let maxCardHeight = 0;
        cards.forEach(card => {
          const rect = card.getBoundingClientRect();
          totalCardsWidth += rect.width;
          if (rect.height > maxCardHeight) maxCardHeight = rect.height;
        });
        reqWidth = totalCardsWidth + (cards.length - 1) * gap + 2 * padding;
        reqHeight = maxCardHeight + 2 * padding;
      } else if (layout === 'vertical') {
        let totalCardsHeight = 0;
        let maxCardWidth = 0;
        cards.forEach(card => {
          const rect = card.getBoundingClientRect();
          totalCardsHeight += rect.height;
          if (rect.width > maxCardWidth) maxCardWidth = rect.width;
        });
        reqWidth = maxCardWidth + 2 * padding;
        reqHeight = totalCardsHeight + (cards.length - 1) * gap + 2 * padding;
      } else if (layout === 'grid') {
        let minX = Infinity, maxX = -Infinity;
        let minY = Infinity, maxY = -Infinity;
        cards.forEach(card => {
          const rect = card.getBoundingClientRect();
          if (rect.left < minX) minX = rect.left;
          if (rect.right > maxX) maxX = rect.right;
          if (rect.top < minY) minY = rect.top;
          if (rect.bottom > maxY) maxY = rect.bottom;
        });
        reqWidth = (maxX - minX) + 2 * padding;
        reqHeight = (maxY - minY) + 2 * padding;
      }

      // 加上 8px 缓冲以应对四舍五入或可能出现的边框/滚动条导致被裁剪
      const finalWidth = Math.ceil(reqWidth) + 8;
      const finalHeight = Math.ceil(reqHeight) + 8;

      try {
        if (window.pywebview && window.pywebview.api && window.pywebview.api.resize_window_to_fit) {
          await pywebview.api.resize_window_to_fit(finalWidth, finalHeight, allowShrink);
        }
      } catch (e) {
        console.warn('调整窗口自适应尺寸失败:', e);
      }
    }

    // === 主题加载 ===
    function loadThemeStyle(style) {
      const link = document.getElementById('theme-style');
      if (link) link.href = 'themes/' + style + '.css';
    }

    // === 初始化 ===
    onMounted(async () => {
      // 循环等待 pywebview.api 初始化完毕，以确保能正确读取到后端保存的配置
      let retries = 0;
      while (!window.pywebview || !window.pywebview.api) {
        await new Promise(resolve => setTimeout(resolve, 50));
        retries++;
        if (retries > 60) break; // 最多等3秒
      }

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
      window.updateMetrics = async (data) => {
        metrics.value = { ...metrics.value, ...data };
        await adjustWindowSize(false); // 指标数据更新不缩小窗口，防止数据波动频繁抖动
      };

      // 暴露 updateConfig 供 Python 热加载调用
      window.updateConfig = async (newConfig) => {
        config.value = { ...config.value, ...newConfig };
        loadThemeStyle(config.value.display.style);
        await adjustWindowSize(true); // 配置发生变更时允许根据新配置收缩或扩大
      };

      // 初始化时执行自适应调整
      await adjustWindowSize(true);
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
