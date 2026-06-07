use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WindowConfig {
    pub width: f64,
    pub height: f64,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub always_on_top: bool,
    pub click_through: bool,
    pub draggable: bool,
    pub opacity: f64,
    pub frameless: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DisplayConfig {
    pub background_color: String,
    pub background_opacity: f64,
    pub blur_radius: f64,
    pub effect_type: String,
    pub layout: String,
    pub custom_css: Option<String>,
    pub font_size: f64,
    pub gap: f64,
    pub padding: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MetricItem {
    pub id: String,
    pub enabled: bool,
    pub label: String,
    pub unit: String,
    pub color: String,
    pub decimals: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub window: WindowConfig,
    pub display: DisplayConfig,
    pub taskbar: TaskbarConfig,
    pub metrics: Vec<MetricItem>,
    pub update_interval_ms: u64,
    pub autostart: bool,
    pub fps_only_in_game: bool,
    pub stop_monitoring_non_game: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskbarConfig {
    pub enabled: bool,
    pub align: String,
    pub offset_x: i32,
    pub offset_y: i32,
    pub font_size: f64,
    pub gap: f64,
    pub padding: f64,
}

impl Default for TaskbarConfig {
    fn default() -> Self {
        TaskbarConfig {
            enabled: true,
            align: "right".to_string(),
            offset_x: 0,
            offset_y: 0,
            font_size: 14.0,
            gap: 12.0,
            padding: 4.0,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            window: WindowConfig {
                width: 400.0,
                height: 60.0,
                x: None,
                y: None,
                always_on_top: true,
                click_through: false,
                draggable: true,
                opacity: 1.0,
                frameless: true,
            },
            display: DisplayConfig {
                background_color: "#1e1e2e".to_string(),
                background_opacity: 0.85,
                blur_radius: 10.0,
                effect_type: "acrylic".to_string(),
                layout: "horizontal".to_string(),
                custom_css: None,
                font_size: 14.0,
                gap: 1.0,
                padding: 8.0,
            },
            taskbar: TaskbarConfig::default(),
            metrics: vec![
                MetricItem { id: "cpu_usage".to_string(), enabled: true, label: "CPU".to_string(), unit: "%".to_string(), color: "#4fc3f7".to_string(), decimals: 0 },
                MetricItem { id: "cpu_power".to_string(), enabled: true, label: "CPUP".to_string(), unit: "W".to_string(), color: "#4fc3f7".to_string(), decimals: 1 },
                MetricItem { id: "cpu_fan".to_string(), enabled: true, label: "CPUF".to_string(), unit: "RPM".to_string(), color: "#4fc3f7".to_string(), decimals: 0 },
                MetricItem { id: "cpu_temp".to_string(), enabled: true, label: "CPUT".to_string(), unit: "°C".to_string(), color: "#4fc3f7".to_string(), decimals: 0 },
                MetricItem { id: "gpu_usage".to_string(), enabled: true, label: "GPU".to_string(), unit: "%".to_string(), color: "#81c784".to_string(), decimals: 0 },
                MetricItem { id: "gpu_temp".to_string(), enabled: true, label: "GPUT".to_string(), unit: "°C".to_string(), color: "#81c784".to_string(), decimals: 0 },
                MetricItem { id: "gpu_power".to_string(), enabled: true, label: "GPUP".to_string(), unit: "W".to_string(), color: "#81c784".to_string(), decimals: 1 },
                MetricItem { id: "gpu_vram".to_string(), enabled: true, label: "VRAM".to_string(), unit: "%".to_string(), color: "#81c784".to_string(), decimals: 0 },
                MetricItem { id: "gpu_fan".to_string(), enabled: true, label: "GPUF".to_string(), unit: "RPM".to_string(), color: "#81c784".to_string(), decimals: 0 },
                MetricItem { id: "ram_usage".to_string(), enabled: true, label: "RAM".to_string(), unit: "%".to_string(), color: "#ffb74d".to_string(), decimals: 0 },
                MetricItem { id: "ram_used".to_string(), enabled: true, label: "RAMU".to_string(), unit: "GB".to_string(), color: "#ffb74d".to_string(), decimals: 1 },
                MetricItem { id: "fps".to_string(), enabled: true, label: "FPS".to_string(), unit: "".to_string(), color: "#ce93d8".to_string(), decimals: 0 },
                MetricItem { id: "fps_1pct_low".to_string(), enabled: true, label: "1%L".to_string(), unit: "".to_string(), color: "#ce93d8".to_string(), decimals: 0 },
            ],
            update_interval_ms: 1000,
            autostart: false,
            fps_only_in_game: true,
            stop_monitoring_non_game: false,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let path = parent.join("config.json");
            if path.exists() {
                return path;
            }
        }
    }
    let cwd_path = PathBuf::from("config.json");
    if cwd_path.exists() {
        return cwd_path;
    }
    if let Ok(exe_path) = env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            return parent.join("config.json");
        }
    }
    PathBuf::from("config.json")
}

pub fn load_config() -> Config {
    let path = get_config_path();
    if path.exists() {
        if let Ok(mut file) = File::open(&path) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&contents) {
                    // 平滑过渡：如果旧版 config.json 没有 "taskbar" 字段，自动从旧的顶层字段迁移数据
                    if value.get("taskbar").is_none() {
                        let enabled = value.get("taskbar_mode").and_then(|v| v.as_bool()).unwrap_or(true);
                        let align = value.get("taskbar_align").and_then(|v| v.as_str()).unwrap_or("right").to_string();
                        let offset_x = value.get("taskbar_offset_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let offset_y = value.get("taskbar_offset_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

                        let taskbar_obj = serde_json::json!({
                            "enabled": enabled,
                            "align": align,
                            "offset_x": offset_x,
                            "offset_y": offset_y,
                            "font_size": 14.0,
                            "gap": 12.0,
                            "padding": 4.0
                        });
                        if let Some(obj) = value.as_object_mut() {
                            obj.insert("taskbar".to_string(), taskbar_obj);
                        }
                    }
                    if let Ok(config) = serde_json::from_value::<Config>(value) {
                        return config;
                    }
                }
            }
        }
    }
    Config::default()
}

pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_config_path();
    let contents = serde_json::to_string_pretty(config)?;
    let mut file = File::create(&path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

pub fn update_autostart_registry(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_SET_VALUE | KEY_WRITE,
    )?;

    if enabled {
        let exe_path = env::current_exe()?;
        let exe_str = exe_path.to_string_lossy();
        run_key.set_value("GyyMonitor", &format!("\"{}\"", exe_str).as_str())?;
        println!("[Registry] Added autostart: {}", exe_str);
    } else {
        match run_key.delete_value("GyyMonitor") {
            Ok(_) => println!("[Registry] Removed autostart"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
