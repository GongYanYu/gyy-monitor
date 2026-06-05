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
        style: 'minimal', layout: 'horizontal',
        custom_css: null, font_size: 14, gap: 18, padding: 8,
      },
      metrics: [],
      update_interval_ms: 1000,
      autostart: false,
    });
    const toastVisible = ref(false);
    let isLoaded = false;

    // === 初始化：从 Python 获取当前配置 ===
    onMounted(async () => {
      // 等待 pywebview.api 完全就绪（包括方法绑定完成）
      // pywebview 官方推荐使用 pywebviewready 事件，它会在 API 方法全部注入后才触发
      await new Promise(resolve => {
        if (window.pywebview && window.pywebview.api && window.pywebview.api.get_config) {
          resolve();
        } else {
          window.addEventListener('pywebviewready', resolve, { once: true });
          // 兜底：若事件未触发（极端情况），5秒后超时继续
          setTimeout(resolve, 5000);
        }
      });

      try {
        const loadedConfig = await pywebview.api.get_config();
        if (loadedConfig) {
          // 深拷贝，防止修改未保存就污染全局
          config.value = JSON.parse(JSON.stringify(loadedConfig));
        }
      } catch (err) {
        console.warn('获取配置失败:', err);
      } finally {
        isLoaded = true;
      }
    });

    // 深度监听配置变化并实时临时应用预览，但不写入磁盘
    watch(config, (newVal) => {
      if (!isLoaded) return;
      if (window.pywebview && window.pywebview.api && window.pywebview.api.apply_config_temp) {
        const rawConfig = JSON.parse(JSON.stringify(newVal));
        pywebview.api.apply_config_temp(rawConfig).catch(err => {
          console.warn('实时预览配置失败:', err);
        });
      }
    }, { deep: true });

    // === 指标排序：向上移动 ===
    function moveMetricUp(index) {
      if (index === 0) return;
      const list = config.value.metrics;
      const temp = list[index];
      list[index] = list[index - 1];
      list[index - 1] = temp;
    }

    // === 指标排序：向下移动 ===
    function moveMetricDown(index) {
      const list = config.value.metrics;
      if (index === list.length - 1) return;
      const temp = list[index];
      list[index] = list[index + 1];
      list[index + 1] = temp;
    }

    // === 保存并应用配置 ===
    async function saveSettings() {
      try {
        // 调用 python 的批量保存接口
        // 因为是双向绑定，config.value 包含用户修改的最新完整数据
        // 直接传递给 set_config_bulk
        await pywebview.api.set_config_bulk(config.value);

        // 弹出保存成功提示
        toastVisible.value = true;
        
        // 1秒后关闭窗口
        setTimeout(() => {
          toastVisible.value = false;
          closeSettings();
        }, 1200);
      } catch (err) {
        console.warn('保存配置失败:', err);
        alert('保存失败，API 错误！');
      }
    }

    // === 关闭设置窗口 ===
    function closeSettings() {
      try {
        if (window.pywebview && window.pywebview.api && window.pywebview.api.close_settings_window) {
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
      moveMetricUp,
      moveMetricDown,
      saveSettings,
      closeSettings,
    };
  }
});

app.mount('#app');
