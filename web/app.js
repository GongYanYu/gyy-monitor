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
    const isFocused = ref(true);

    // 拖动相关（纯 Web 兜底用，Tauri 下会使用更顺滑的原生 drag）
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
          background_color: '#1e1e2e',
          background_opacity: 0.85,
          blur_radius: 10.0,
          effect_type: 'acrylic',
          layout: 'horizontal',
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

    function hexToRgba(hex, opacity) {
      if (!hex) return `rgba(30, 30, 46, ${opacity})`;
      let c = hex.substring(1);
      if (c.length === 3) {
        c = c[0] + c[0] + c[1] + c[1] + c[2] + c[2];
      }
      const r = parseInt(c.substring(0, 2), 16);
      const g = parseInt(c.substring(2, 4), 16);
      const b = parseInt(c.substring(4, 6), 16);
      return `rgba(${r}, ${g}, ${b}, ${opacity})`;
    }

    const containerStyle = computed(() => {
      const d = config.value.display;
      const effect = d.effect_type || 'none';
      
      // 1. 动态确定背景不透明度与是否应用 CSS 模糊
      let bgOpacity = d.background_opacity ?? 0.85;
      let applyCssBlur = false;

      if (effect === 'acrylic' || effect === 'mica') {
        if (isFocused.value) {
          // 获得焦点且有原生特效：极高透明度，由系统渲染毛玻璃，禁用 CSS 模糊以防冲突和卡顿
          bgOpacity = Math.min(bgOpacity, 0.1);
          applyCssBlur = false;
        } else {
          // 失去焦点：Rust 端会清除原生特效以防黑屏，此时降级为纯 CSS 模糊和透明度，完全遵循用户设置
          applyCssBlur = true;
        }
      } else {
        // 无原生特效：始终使用 CSS 模糊
        applyCssBlur = true;
      }

      const bgColor = d.background_color ?? '#1e1e2e';
      const blur = d.blur_radius ?? 10.0;
      
      const rgbaBg = hexToRgba(bgColor, bgOpacity);
      const hasBgOrBlur = bgOpacity > 0 || blur > 0;
      
      // 决定最终的 blur 滤镜字符串
      const blurStr = (applyCssBlur && blur > 0) ? `blur(${blur}px)` : 'none';

      return {
        padding: (d.padding || 8) + 'px',
        fontSize: (d.font_size || 14) + 'px',
        background: rgbaBg,
        backdropFilter: blurStr,
        webkitBackdropFilter: blurStr,
        borderColor: hasBgOrBlur ? 'rgba(255, 255, 255, 0.08)' : 'transparent',
        boxShadow: hasBgOrBlur ? '0 8px 32px rgba(0, 0, 0, 0.3)' : 'none',
      };
    });

    const contentStyle = computed(() => {
      const d = config.value.display;
      return {
        gap: (d.gap || 18) + 'px',
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

    function layoutLabel(l) {
      return { horizontal: '横向', vertical: '纵向', grid: '网格' }[l] || l;
    }

    // 显示右键上下文菜单（如果不是原生托盘）
    async function showMenu(e) {
      // 阻止浏览器默认右键菜单
      e.preventDefault();
      if (window.__TAURI__) {
        try {
          await window.__TAURI__.core.invoke('show_context_menu');
        } catch (err) {
          console.warn('打开原生右键菜单失败:', err);
        }
      } else {
        menuX.value = e.clientX;
        menuY.value = e.clientY;
        menuVisible.value = true;
      }
    }

    async function toggleClickThrough() {
      const newVal = !config.value.window.click_through;
      config.value.window.click_through = newVal;
      menuVisible.value = false;
      try {
        if (window.__TAURI__) {
          await window.__TAURI__.core.invoke('set_config', { key: 'window.click_through', value: newVal });
        } else if (window.pywebview) {
          await pywebview.api.set_config('window.click_through', newVal);
          await pywebview.api.toggle_click_through(newVal);
        }
      } catch (e) { console.warn('Bridge 调用失败:', e); }
    }

    async function setLayout(layout) {
      config.value.display.layout = layout;
      menuVisible.value = false;
      try {
        if (window.__TAURI__) {
          await window.__TAURI__.core.invoke('set_config', { key: 'display.layout', value: layout });
        } else if (window.pywebview) {
          await pywebview.api.set_config('display.layout', layout);
        }
      } catch (e) { console.warn('Bridge 调用失败:', e); }
    }

    async function openConfig() {
      menuVisible.value = false;
      try {
        if (window.__TAURI__) {
          await window.__TAURI__.core.invoke('open_config_file');
        } else if (window.pywebview) {
          await pywebview.api.open_config_file();
        }
      } catch (e) { console.warn('Bridge 调用失败:', e); }
    }

    async function hideWindow() {
      menuVisible.value = false;
      try {
        if (window.__TAURI__) {
          await window.__TAURI__.core.invoke('toggle_visible');
        } else if (window.pywebview) {
          await pywebview.api.toggle_visible();
        }
      } catch (e) { console.warn('Bridge 调用失败:', e); }
    }

    // === 拖动支持 ===
    function onMouseDown(e) {
      if (!config.value.window.draggable) return;
      if (e.target.closest('.context-menu')) return;
      
      if (window.__TAURI__) {
        try {
          window.__TAURI__.core.invoke('start_drag');
        } catch (err) {
          console.warn('调用 start_drag 失败:', err);
        }
      } else {
        dragging = true;
        dragStartX = e.screenX;
        dragStartY = e.screenY;
      }
    }

    function onMouseMove(e) {
      if (!dragging) return;
      const dx = e.screenX - dragStartX;
      const dy = e.screenY - dragStartY;
      dragStartX = e.screenX;
      dragStartY = e.screenY;
      try {
        if (window.pywebview) {
          pywebview.api.move_window(dx, dy);
        }
      } catch (e) { /* ignore */ }
    }

    // 切换设置面板
    async function openSettings() {
      menuVisible.value = false;
      try {
        if (window.__TAURI__) {
          await window.__TAURI__.core.invoke('open_settings_window');
        } else if (window.pywebview) {
          await pywebview.api.open_settings_window();
        }
      } catch (e) {
        console.warn('打开设置界面失败:', e);
      }
    }

    function onMouseUp() {
      dragging = false;
    }
    // 窗口大小现已完全由内容和全透明窗口容器自适应撑开

    // === 初始化 ===
    onMounted(async () => {
      // 监听窗口焦点状态
      window.addEventListener('focus', () => {
        isFocused.value = true;
      });
      window.addEventListener('blur', () => {
        isFocused.value = false;
      });

      // 1. 如果是 Tauri 环境
      if (window.__TAURI__) {
        const { invoke } = window.__TAURI__.core;
        const { listen } = window.__TAURI__.event;

        // 加载配置
        try {
          const cfg = await invoke('get_config');
          if (cfg) config.value = cfg;
        } catch (e) {
          console.warn('无法从后端加载配置，使用默认值:', e);
        }

        // 监听来自 Rust 的指标高频推送
        listen('metrics-update', async (event) => {
          metrics.value = { ...metrics.value, ...event.payload };
        });

        // 监听来自 Rust 的配置更新（托盘菜单/设置窗口触发的修改）
        listen('config-changed', async (event) => {
          config.value = { ...config.value, ...event.payload };
        });

        // 监听内容组件大小并自适应窗口物理尺寸
        const content = document.querySelector('.monitor-content');
        if (content) {
          const resizeObserver = new ResizeObserver((entries) => {
            for (let entry of entries) {
              const rect = entry.target.getBoundingClientRect();
              // 加上内外边距和边框边距
              const padding = (config.value.display.padding || 8) * 2;
              const width = Math.max(100, Math.ceil(rect.width) + padding + 2);
              const height = Math.max(30, Math.ceil(rect.height) + padding + 2);
              
              invoke('resize_window_to_fit', { width, height }).catch((err) => {
                console.warn('调整窗口大小失败:', err);
              });
            }
          });
          resizeObserver.observe(content);
        }

      } else {
        // 2. 否则，如果是 pywebview 环境（兜底）
        await new Promise(resolve => {
          if (window.pywebview && window.pywebview.api && window.pywebview.api.get_config) {
            resolve();
          } else {
            window.addEventListener('pywebviewready', resolve, { once: true });
            setTimeout(resolve, 5000);
          }
        });

        try {
          const cfg = await pywebview.api.get_config();
          if (cfg) config.value = cfg;
        } catch (e) {
          console.warn('无法从后端加载配置，使用默认值:', e);
        }

        // 暴露 updateMetrics 供 Python 调用
        window.updateMetrics = async (data) => {
          metrics.value = { ...metrics.value, ...data };
        };

        // 暴露 updateConfig 供 Python 热加载调用
        window.updateConfig = async (newConfig) => {
          config.value = { ...config.value, ...newConfig };
          loadThemeStyle(config.value.display.style);
        };
      }

      // 注册全局鼠标事件用于拖动（仅纯 Web/pywebview 环境有效）
      document.addEventListener('mousemove', onMouseMove);
      document.addEventListener('mouseup', onMouseUp);

      // 关闭自定义菜单的全局监听
      document.addEventListener('click', () => {
        menuVisible.value = false;
      });
    });

    onUnmounted(() => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      if (window.updateMetrics) delete window.updateMetrics;
      if (window.updateConfig) delete window.updateConfig;
    });

    return {
      config, metrics, menuVisible, menuX, menuY,
      enabledMetrics, containerStyle, contentStyle, cardStyle,
      formatValue, layoutLabel,
      showMenu, toggleClickThrough, setLayout,
      openConfig, hideWindow, onMouseDown, openSettings
    };
  },
});

app.mount('#app');
