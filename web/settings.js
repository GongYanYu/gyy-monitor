/* gyy-monitor 设置页 — Vue 3 逻辑 */

const { createApp, ref, onMounted, watch } = Vue;

const app = createApp({
  setup() {
    const currentTab = ref('display');
    const config = ref({
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
      taskbar: {
        enabled: true,
        align: 'right',
        offset_x: 0,
        offset_y: 0,
        font_size: 14,
        gap: 12,
        padding: 4,
      },
      metrics: [],
      update_interval_ms: 1000,
      autostart: false,
      fps_only_in_game: true,
      stop_monitoring_non_game: false,
    });
    const toastVisible = ref(false);
    let isLoaded = false;

    // === 初始化：从后端获取当前配置 ===
    onMounted(async () => {
      if (window.__TAURI__) {
        // Tauri 环境
        const { invoke } = window.__TAURI__.core;
        try {
          const loadedConfig = await invoke('get_config');
          if (loadedConfig) {
            config.value = JSON.parse(JSON.stringify(loadedConfig));
          }
        } catch (err) {
          console.warn('Tauri 获取配置失败:', err);
        } finally {
          isLoaded = true;
        }
      } else {
        // pywebview 兜底环境
        await new Promise(resolve => {
          if (window.pywebview && window.pywebview.api && window.pywebview.api.get_config) {
            resolve();
          } else {
            window.addEventListener('pywebviewready', resolve, { once: true });
            setTimeout(resolve, 5000);
          }
        });

        try {
          const loadedConfig = await pywebview.api.get_config();
          if (loadedConfig) {
            config.value = JSON.parse(JSON.stringify(loadedConfig));
          }
        } catch (err) {
          console.warn('pywebview 获取配置失败:', err);
        } finally {
          isLoaded = true;
        }
      }
    });

    // 深度监听配置变化并实时临时应用预览，但不写入磁盘
    watch(config, (newVal) => {
      if (!isLoaded) return;
      const rawConfig = JSON.parse(JSON.stringify(newVal));
      if (window.__TAURI__) {
        window.__TAURI__.core.invoke('apply_config_temp', { patch: rawConfig }).catch(err => {
          console.warn('Tauri 实时预览配置失败:', err);
        });
      } else if (window.pywebview && window.pywebview.api && window.pywebview.api.apply_config_temp) {
        pywebview.api.apply_config_temp(rawConfig).catch(err => {
          console.warn('pywebview 实时预览配置失败:', err);
        });
      }
    }, { deep: true });

    // === 指标排序：向上移动 ===
    fn_moveUp = (index) => {
      if (index === 0) return;
      const list = config.value.metrics;
      const temp = list[index];
      list[index] = list[index - 1];
      list[index - 1] = temp;
    };

    // === 指标排序：向下移动 ===
    fn_moveDown = (index) => {
      const list = config.value.metrics;
      if (index === list.length - 1) return;
      const temp = list[index];
      list[index] = list[index + 1];
      list[index + 1] = temp;
    };

    // === 保存并应用配置 ===
    async function saveSettings() {
      try {
        const rawConfig = JSON.parse(JSON.stringify(config.value));
        if (window.__TAURI__) {
          await window.__TAURI__.core.invoke('set_config_bulk', { patch: rawConfig });
        } else {
          await pywebview.api.set_config_bulk(rawConfig);
        }

        // 弹出保存成功提示
        toastVisible.value = true;
        
        // 1.2秒后仅隐藏保存提示，不关闭窗口
        setTimeout(() => {
          toastVisible.value = false;
        }, 1200);
      } catch (err) {
        console.warn('保存配置失败:', err);
        alert('保存失败，API 错误！');
      }
    }

    // === 关闭设置窗口 ===
    function closeSettings() {
      try {
        if (window.__TAURI__) {
          window.__TAURI__.core.invoke('close_settings_window');
        } else if (window.pywebview && window.pywebview.api && window.pywebview.api.close_settings_window) {
          pywebview.api.close_settings_window();
        } else {
          window.close();
        }
      } catch (err) {
        console.warn('关闭窗口失败:', err);
        window.close();
      }
    }

    return {
      currentTab,
      config,
      toastVisible,
      moveMetricUp: fn_moveUp,
      moveMetricDown: fn_moveDown,
      saveSettings,
      closeSettings,
    };
  }
});

app.mount('#app');
