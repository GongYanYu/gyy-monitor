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
    pub metrics: Vec<MetricItem>,
    pub update_interval_ms: u64,
    pub autostart: bool,
    pub fps_only_in_game: bool,
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
                gap: 18.0,
                padding: 8.0,
            },
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
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
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
