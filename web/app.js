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
