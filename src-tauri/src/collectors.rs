use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use sysinfo::System;
use nvml_wrapper::Nvml;
use wmi::{COMLibrary, WMIConnection};

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

// Windows FFI imports
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Diagnostics::Etw::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use windows_sys::Win32::UI::Shell::IsUserAnAdmin;

const WNODE_FLAG_TRACING_SHARE_SESSION: u32 = 0x00010000;

// Global structures for ETW DXGI event tracking
static FRAME_TIMESTAMPS: LazyLock<Mutex<HashMap<(u32, u64), Vec<Instant>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static EVENT_TIMESTAMPS: LazyLock<Mutex<HashMap<u32, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const ETW_SESSION_NAME: &str = "GyyMonitor-DXGI-FPS";

// Structs for WMI deserialization
#[derive(Deserialize, Debug)]
struct ThermalZoneInfo {
    #[serde(rename = "HighPrecisionTemperature")]
    high_precision_temperature: Option<u64>,
    #[serde(rename = "Temperature")]
    temperature: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct EnergyMeter {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Power")]
    power: u64,
}

#[derive(Deserialize, Debug)]
struct LhmSensor {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Value")]
    value: f32,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct MetricsSnapshot {
    pub cpu_usage: Option<f64>,
    pub cpu_temp: Option<f64>,
    pub cpu_power: Option<f64>,
    pub cpu_fan: Option<f64>,
    pub gpu_usage: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub gpu_power: Option<f64>,
    pub gpu_vram: Option<f64>,
    pub gpu_fan: Option<f64>,
    pub ram_usage: Option<f64>,
    pub ram_used: Option<f64>,
    pub ram_total: Option<f64>,
    pub fps: Option<f64>,
    pub fps_1pct_low: Option<f64>,
    pub is_system_dark: Option<bool>,
}

pub struct SystemCollector {
    sys: System,
    nvml: Option<Nvml>,
    com_lib: Option<COMLibrary>,
    etw_active: bool,
}

impl SystemCollector {
    pub fn new() -> Self {
        // Initialize sysinfo
        let mut sys = System::new_all();
        sys.refresh_cpu();
        sys.refresh_memory();

        // Initialize NVML
        let nvml = Nvml::init().ok();

        // Initialize COM library for WMI (one-time on this thread)
        let com_lib = COMLibrary::new().ok();

        // Start ETW session for FPS in background
        let mut etw_active = false;
        unsafe {
            if is_admin() {
                start_etw_session();
                etw_active = true;
            } else {
                println!("[FPS] Not running as administrator, ETW FPS collection is disabled.");
            }
        }

        SystemCollector {
            sys,
            nvml,
            com_lib,
            etw_active,
        }
    }

    pub fn collect(&mut self) -> MetricsSnapshot {
        let mut snapshot = MetricsSnapshot::default();

        // 1. Refresh & Collect Basic CPU and RAM metrics
        self.sys.refresh_cpu();
        self.sys.refresh_memory();

        // sysinfo 0.30 global_cpu_info works directly
        if let Some(_cpu) = self.sys.cpus().first() {
            // Note: global cpu info can also be queried but global_cpu_info().cpu_usage() is standard.
            // In sysinfo 0.30: sys.global_cpu_info()
            snapshot.cpu_usage = Some(self.sys.global_cpu_info().cpu_usage() as f64);
        }
        
        let total_mem = self.sys.total_memory() as f64;
        let used_mem = self.sys.used_memory() as f64;
        if total_mem > 0.0 {
            snapshot.ram_usage = Some((used_mem / total_mem) * 100.0);
        }
        snapshot.ram_used = Some(used_mem / (1024.0 * 1024.0 * 1024.0));
        snapshot.ram_total = Some(total_mem / (1024.0 * 1024.0 * 1024.0));

        // 2. Collect CPU Temperature, Power, Fan via WMI
        if let Some(com) = &self.com_lib {
            if let Ok(wmi_con) = WMIConnection::new(*com) {
                // CPU Temp
                snapshot.cpu_temp = self.collect_cpu_temp(&wmi_con);
                // CPU Power
                snapshot.cpu_power = self.collect_cpu_power(&wmi_con);
                // CPU Fan
                snapshot.cpu_fan = self.collect_cpu_fan(&wmi_con, *com);
            }
        }

        // 3. Collect NVIDIA GPU metrics
        if let Some(nvml) = &self.nvml {
            if let Ok(device) = nvml.device_by_index(0) {
                snapshot.gpu_usage = device.utilization_rates().map(|u| u.gpu as f64).ok();
                snapshot.gpu_temp = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu).map(|t| t as f64).ok();
                snapshot.gpu_power = device.power_usage().map(|p| (p as f64) / 1000.0).ok();
                snapshot.gpu_vram = device.memory_info().map(|m| (m.used as f64 / m.total as f64) * 100.0).ok();
                snapshot.gpu_fan = device.fan_speed(0).map(|f| f as f64).ok();
            }
        }

        // 4. Collect FPS via ETW
        if self.etw_active {
            let (fps, fps_low) = self.collect_fps();
            snapshot.fps = Some(fps);
            snapshot.fps_1pct_low = Some(fps_low);
        }

        // 5. Collect Windows System Theme (Dark/Light)
        #[cfg(target_os = "windows")]
        {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(theme_key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize") {
                if let Ok(val) = theme_key.get_value::<u32, _>("SystemUsesLightTheme") {
                    snapshot.is_system_dark = Some(val == 0);
                }
            }
        }

        snapshot
    }

    fn collect_cpu_temp(&self, wmi_con: &WMIConnection) -> Option<f64> {
        let query = "SELECT HighPrecisionTemperature, Temperature FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation";
        if let Ok(results) = wmi_con.raw_query::<ThermalZoneInfo>(query) {
            let mut temps = Vec::new();
            for item in results {
                if let Some(hpt) = item.high_precision_temperature {
                    let c = (hpt as f64 / 10.0) - 273.15;
                    if c > 0.0 && c < 150.0 {
                        temps.push(c);
                    }
                } else if let Some(t) = item.temperature {
                    let c = t as f64 - 273.15;
                    if c > 0.0 && c < 150.0 {
                        temps.push(c);
                    }
                }
            }
            if !temps.is_empty() {
                return temps.into_iter().max_by(|a, b| a.partial_cmp(b).unwrap());
            }
        }
        None
    }

    fn collect_cpu_power(&self, wmi_con: &WMIConnection) -> Option<f64> {
        let query = "SELECT Name, Power FROM Win32_PerfFormattedData_PowerMeterCounter_EnergyMeter";
        if let Ok(results) = wmi_con.raw_query::<EnergyMeter>(query) {
            for item in results {
                if item.name.contains("RAPL_Package0_PKG") {
                    return Some(item.power as f64 / 1000.0);
                }
            }
            // Fallback: Max power in the list
            let mut powers = Vec::new();
            let query_all = "SELECT Power FROM Win32_PerfFormattedData_PowerMeterCounter_EnergyMeter";
            if let Ok(results_all) = wmi_con.raw_query::<EnergyMeter>(query_all) {
                for item in results_all {
                    powers.push(item.power as f64 / 1000.0);
                }
            }
            if !powers.is_empty() {
                return powers.into_iter().max_by(|a, b| a.partial_cmp(b).unwrap());
            }
        }
        None
    }

    fn collect_cpu_fan(&self, _wmi_con: &WMIConnection, com: COMLibrary) -> Option<f64> {
        // Try LibreHardwareMonitor
        if let Ok(wmi_lhm) = WMIConnection::with_namespace_path("ROOT\\LibreHardwareMonitor", com) {
            let query = "SELECT Name, Value FROM Sensor WHERE SensorType = 'Fan'";
            if let Ok(results) = wmi_lhm.raw_query::<LhmSensor>(query) {
                let mut fans = Vec::new();
                for item in results {
                    if item.name.to_lowercase().contains("cpu") {
                        return Some(item.value as f64);
                    }
                    fans.push(item.value as f64);
                }
                if !fans.is_empty() {
                    return fans.into_iter().max_by(|a, b| a.partial_cmp(b).unwrap());
                }
            }
        }

        // Try OpenHardwareMonitor
        if let Ok(wmi_ohm) = WMIConnection::with_namespace_path("ROOT\\OpenHardwareMonitor", com) {
            let query = "SELECT Name, Value FROM Sensor WHERE SensorType = 'Fan'";
            if let Ok(results) = wmi_ohm.raw_query::<LhmSensor>(query) {
                let mut fans = Vec::new();
                for item in results {
                    if item.name.to_lowercase().contains("cpu") {
                        return Some(item.value as f64);
                    }
                    fans.push(item.value as f64);
                }
                if !fans.is_empty() {
                    return fans.into_iter().max_by(|a, b| a.partial_cmp(b).unwrap());
                }
            }
        }

        None
    }

    fn collect_fps(&self) -> (f64, f64) {
        let active_pid = get_active_window_pid();
        if active_pid == 0 {
            return (0.0, 0.0);
        }

        let now = Instant::now();

        // Check timeout: if no events in 1.5 seconds, FPS is 0
        if let Ok(event_times) = EVENT_TIMESTAMPS.lock() {
            if let Some(&last_time) = event_times.get(&active_pid) {
                if now.duration_since(last_time) > Duration::from_millis(1500) {
                    return (0.0, 0.0);
                }
            } else {
                return (0.0, 0.0);
            }
        }

        if let Ok(mut timestamps_map) = FRAME_TIMESTAMPS.lock() {
            let active_keys: Vec<(u32, u64)> = timestamps_map
                .keys()
                .filter(|k| k.0 == active_pid)
                .cloned()
                .collect();

            let mut swapchain_results = Vec::new();
            for key in active_keys {
                if let Some(timestamps) = timestamps_map.get_mut(&key) {
                    // Filter timestamps within last 1.5 seconds
                    let cutoff = now - Duration::from_millis(1500);
                    timestamps.retain(|&t| t >= cutoff);

                    if timestamps.len() < 2 {
                        continue;
                    }

                    let total_frames = timestamps.len();
                    let time_span = timestamps.last().unwrap().duration_since(timestamps[0]).as_secs_f64();
                    
                    let fps = if time_span > 0.0 {
                        (total_frames - 1) as f64 / time_span
                    } else {
                        0.0
                    };

                    // Compute 1% Low FPS
                    let mut frame_times = Vec::new();
                    for i in 1..timestamps.len() {
                        frame_times.push(timestamps[i].duration_since(timestamps[i - 1]).as_secs_f64());
                    }

                    let fps_1pct_low = if frame_times.len() >= 10 {
                        frame_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let num_1pct = std::cmp::max(1, frame_times.len() / 100);
                        let slowest = &frame_times[frame_times.len() - num_1pct..];
                        let avg_slowest: f64 = slowest.iter().sum::<f64>() / slowest.len() as f64;
                        if avg_slowest > 0.0 {
                            1.0 / avg_slowest
                        } else {
                            0.0
                        }
                    } else {
                        fps
                    };

                    swapchain_results.push((fps, fps_1pct_low));
                }
            }

            if let Some(best) = swapchain_results.into_iter().max_by(|a, b| a.0.partial_cmp(&b.0).unwrap()) {
                return (best.0.round(), best.1.round());
            }
        }

        (0.0, 0.0)
    }
}

// Check admin rights for ETW
unsafe fn is_admin() -> bool {
    IsUserAnAdmin() != 0
}

fn get_active_window_pid() -> u32 {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return 0;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        pid
    }
}

// ETW Event callback
unsafe extern "system" fn event_record_callback(record: *mut EVENT_RECORD) {
    if record.is_null() {
        return;
    }
    let rec = &*record;

    // DXGI Present Start Event ID is 42
    if rec.EventHeader.EventDescriptor.Id != 42 {
        return;
    }

    let pid = rec.EventHeader.ProcessId;
    if pid == 0 {
        return;
    }

    let mut swap_chain: u64 = 0;
    if rec.UserDataLength >= 8 {
        std::ptr::copy_nonoverlapping(rec.UserData as *const u8, &mut swap_chain as *mut u64 as *mut u8, 8);
    }

    let now = Instant::now();
    let key = (pid, swap_chain);

    if let Ok(mut timestamps) = FRAME_TIMESTAMPS.lock() {
        let list = timestamps.entry(key).or_insert_with(Vec::new);
        list.push(now);

        let cutoff = now - Duration::from_secs(3);
        list.retain(|&t| t >= cutoff);
    }

    if let Ok(mut event_times) = EVENT_TIMESTAMPS.lock() {
        event_times.insert(pid, now);
    }
}

// Setup and start ETW DXGI trace session
unsafe fn start_etw_session() {
    let session_name_w: Vec<u16> = ETW_SESSION_NAME.encode_utf16().chain(std::iter::once(0)).collect();

    // Size of properties structure + session name string buffer
    let size = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + session_name_w.len() * 2;
    let mut buffer = vec![0u8; size];

    let props = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
    (*props).Wnode.BufferSize = size as u32;
    (*props).Wnode.Flags = WNODE_FLAG_TRACING_SHARE_SESSION;
    (*props).Wnode.Guid = windows_sys::core::GUID {
        data1: 0,
        data2: 0,
        data3: 0,
        data4: [0; 8],
    };
    (*props).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
    (*props).LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;

    // Copy session name to the offset
    let dest_ptr = buffer.as_mut_ptr().add((*props).LoggerNameOffset as usize) as *mut u16;
    std::ptr::copy_nonoverlapping(session_name_w.as_ptr(), dest_ptr, session_name_w.len());

    // Stop existing session first (cleanup)
    let trace_handle_stop = CONTROLTRACE_HANDLE { Value: 0 };
    ControlTraceW(trace_handle_stop, session_name_w.as_ptr(), props, EVENT_TRACE_CONTROL_STOP);

    // Start new trace session
    let mut session_handle = CONTROLTRACE_HANDLE { Value: 0 };
    let status = StartTraceW(&mut session_handle, session_name_w.as_ptr(), props);
    if status != ERROR_SUCCESS {
        println!("[FPS] StartTraceW failed: {}", status);
        return;
    }

    // Enable Microsoft-Windows-DXGI provider
    // GUID: {ca11c036-0102-4a2d-a6ad-f03cfed5d3c9}
    let dxgi_guid = windows_sys::core::GUID {
        data1: 0xca11c036,
        data2: 0x0102,
        data3: 0x4a2d,
        data4: [0xa6, 0xad, 0xf0, 0x3c, 0xfe, 0xd5, 0xd3, 0xc9],
    };

    let status = EnableTraceEx2(
        session_handle,
        &dxgi_guid,
        EVENT_CONTROL_CODE_ENABLE_PROVIDER,
        TRACE_LEVEL_INFORMATION as u8,
        0,
        0,
        0,
        std::ptr::null(),
    );
    if status != ERROR_SUCCESS {
        println!("[FPS] EnableTraceEx2 failed: {}", status);
        return;
    }

    // Open Trace in real-time mode
    let mut log_file: EVENT_TRACE_LOGFILEW = std::mem::zeroed();
    log_file.LoggerName = session_name_w.as_ptr() as *mut u16;
    log_file.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
    log_file.Anonymous2.EventRecordCallback = Some(event_record_callback);

    let opened_handle = OpenTraceW(&mut log_file);
    if opened_handle.Value == !0 { // INVALID_PROCESSTRACE_HANDLE is all ones
        println!("[FPS] OpenTraceW failed");
        return;
    }

    // Process Trace in background thread
    thread::spawn(move || {
        let status = ProcessTrace(&opened_handle, 1, std::ptr::null(), std::ptr::null());
        println!("[FPS] ProcessTrace thread exited with status: {}", status);
        CloseTrace(opened_handle);
    });
}
