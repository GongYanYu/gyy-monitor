mod config;
mod app_state;
mod collectors;
mod commands;

use tauri::Manager;
use tauri::Emitter;
use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use std::time::Duration;
use std::thread;
use std::sync::atomic::Ordering;

use crate::app_state::AppState;
use crate::config::{load_config, update_autostart_registry};

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, FindWindowExW, SetParent, SetWindowLongW, GetWindowLongW,
    SetWindowPos, GWL_STYLE, WS_CHILD, WS_POPUP, SWP_NOZORDER, SWP_SHOWWINDOW,
    GetForegroundWindow, GetWindowRect, GetSystemMetrics, GetClassNameW,
    SM_CXSCREEN, SM_CYSCREEN
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;

pub fn get_hwnd_from_window<W: raw_window_handle::HasWindowHandle>(window: &W) -> Option<isize> {
    use raw_window_handle::RawWindowHandle;
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
            return Some(win32_handle.hwnd.get());
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn to_wstring(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
pub unsafe fn embed_in_taskbar(window_hwnd: HWND, width: i32, height: i32, align: &str, offset_x: i32, offset_y: i32) -> Result<(), String> {
    let shell_tray = FindWindowW(to_wstring("Shell_TrayWnd").as_ptr(), std::ptr::null());
    if shell_tray == 0 {
        return Err("Could not find Shell_TrayWnd".into());
    }

    // 修改窗口样式为子窗口且无弹出属性
    let style = GetWindowLongW(window_hwnd, GWL_STYLE);
    SetWindowLongW(window_hwnd, GWL_STYLE, (style & !(WS_POPUP as i32)) | WS_CHILD as i32);
    
    SetParent(window_hwnd, shell_tray);

    reposition_taskbar_window_hwnd(window_hwnd, width, height, align, offset_x, offset_y);
    
    Ok(())
}

#[cfg(target_os = "windows")]
pub unsafe fn reposition_taskbar_window_hwnd(window_hwnd: HWND, width: i32, height: i32, align: &str, offset_x: i32, offset_y: i32) {
    let shell_tray = FindWindowW(to_wstring("Shell_TrayWnd").as_ptr(), std::ptr::null());
    if shell_tray == 0 { return; }
    
    // 动态获取任务栏高度，计算垂直居中的 y 坐标
    let mut shell_rect = std::mem::zeroed();
    let taskbar_height = if windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(shell_tray, &mut shell_rect) != 0 {
        shell_rect.bottom - shell_rect.top
    } else {
        40 // 默认兜底高度
    };
    let y = (taskbar_height - height) / 2;
    
    let tray_notify = FindWindowExW(shell_tray, 0, to_wstring("TrayNotifyWnd").as_ptr(), std::ptr::null());
    let mut tray_rect = std::mem::zeroed();
    
    let x = if align == "left" {
        60 + offset_x // 左侧位置，避开 Windows 徽标键/小组件并应用微调
    } else {
        // 右侧位置，放置在系统托盘 TrayNotifyWnd 左侧
        if tray_notify != 0 && windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(tray_notify, &mut tray_rect) != 0 {
            let mut pt = windows_sys::Win32::Foundation::POINT { x: tray_rect.left, y: tray_rect.top };
            ScreenToClient(shell_tray, &mut pt);
            pt.x - width - 10 + offset_x // 留出 10px 间距并加上水平微调
        } else {
            800 + offset_x // 默认右侧兜底并加上水平微调
        }
    };
    let y = y + offset_y; // 加上垂直微调
    
    SetWindowPos(window_hwnd, 0, x, y, width, height, SWP_NOZORDER | SWP_SHOWWINDOW);
}

#[cfg(target_os = "windows")]
fn is_foreground_window_fullscreen() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return false;
        }

        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) != 0 {
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);

            if width >= screen_width && height >= screen_height {
                let mut class_name = [0u16; 256];
                let len = GetClassNameW(hwnd, class_name.as_mut_ptr(), class_name.len() as i32);
                if len > 0 {
                    let name = String::from_utf16_lossy(&class_name[..len as usize]);
                    if name == "Progman" || name == "WorkerW" || name == "Shell_TrayWnd" {
                        return false;
                    }
                }
                return true;
            }
        }
        false
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_config = load_config();
    let _ = update_autostart_registry(initial_config.autostart);

    tauri::Builder::default()
        .manage(AppState::new(initial_config))
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config,
            commands::set_config_bulk,
            commands::apply_config_temp,
            commands::resize_window_to_fit,
            commands::toggle_click_through,
            commands::move_window,
            commands::toggle_visible,
            commands::open_config_file,
            commands::read_file,
            commands::open_settings_window,
            commands::close_settings_window,
            commands::quit_app,
            commands::show_context_menu,
            commands::start_drag,
            commands::get_metrics
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            let is_game = {
                #[cfg(target_os = "windows")]
                {
                    is_foreground_window_fullscreen()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            };
            
            let state = app.state::<AppState>();
            state.is_game_active.store(is_game, Ordering::Relaxed);
            let config = state.config.lock().unwrap().clone();

            // Setup background thread for metric collection
            let thread_app = app_handle.clone();
            thread::spawn(move || {
                let mut collector = collectors::SystemCollector::new();
                loop {
                    let state = thread_app.state::<AppState>();
                    let is_game = state.is_game_active.load(Ordering::Relaxed);
                    let (interval, stop_non_game) = {
                        let config = state.config.lock().unwrap();
                        (config.update_interval_ms, config.stop_monitoring_non_game)
                    };

                    if stop_non_game && !is_game {
                        // 非游戏状态下且开启了停止监控，则跳过此次采集
                        thread::sleep(Duration::from_millis(interval));
                        continue;
                    }

                    let snapshot = collector.collect();

                    // Update global state metrics
                    let snapshot_val = serde_json::to_value(&snapshot).unwrap_or(serde_json::json!({}));
                    {
                        let mut metrics = state.metrics.lock().unwrap();
                        *metrics = snapshot_val;
                    }

                    // Push metrics to webview windows
                    if let Some(main_win) = thread_app.get_webview_window("main") {
                        let _ = main_win.emit("metrics-update", &snapshot);
                    }
                    if let Some(taskbar_win) = thread_app.get_webview_window("taskbar") {
                        let _ = taskbar_win.emit("metrics-update", &snapshot);
                    }
                    if let Some(settings_win) = thread_app.get_webview_window("settings") {
                        let _ = settings_win.emit("metrics-update", &snapshot);
                    }

                    thread::sleep(Duration::from_millis(interval));
                }
            });

            // 启动后台前台窗口全屏检测线程
            let thread_app_game = app_handle.clone();
            thread::spawn(move || {
                let mut last_state = is_game;
                loop {
                    #[cfg(target_os = "windows")]
                    {
                        let is_game_active = is_foreground_window_fullscreen();
                        let state = thread_app_game.state::<AppState>();
                        state.is_game_active.store(is_game_active, Ordering::Relaxed);
                        
                        if is_game_active != last_state {
                            last_state = is_game_active;
                            
                            // 切换窗口可见性
                            let config = state.config.lock().unwrap().clone();
                            
                            if let Some(main_win) = thread_app_game.get_webview_window("main") {
                                if is_game_active || !config.taskbar.enabled {
                                    let _ = main_win.show();
                                    let _ = main_win.set_focus();
                                } else {
                                    let _ = main_win.hide();
                                }
                            }
                            
                            if let Some(taskbar_win) = thread_app_game.get_webview_window("taskbar") {
                                if config.taskbar.enabled && !is_game_active {
                                    let _ = taskbar_win.show();
                                } else {
                                    let _ = taskbar_win.hide();
                                }
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(1000));
                }
            });

            // Initialize main window
            if let Some(main_win) = app.get_webview_window("main") {
                // Position, Always on top, Click through
                if let (Some(x), Some(y)) = (config.window.x, config.window.y) {
                    let _ = main_win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                        x: x as i32,
                        y: y as i32,
                    }));
                }
                let _ = main_win.set_always_on_top(config.window.always_on_top);
                let _ = main_win.set_ignore_cursor_events(config.window.click_through);

                // Apply Acrylic/Mica effects on Windows
                #[cfg(target_os = "windows")]
                {
                    crate::commands::apply_vibrancy(&main_win, &config.display.effect_type);

                    // 监听窗口焦点变化，动态应用/清除原生组合效果以防止失焦时产生黑块
                    let win_clone = main_win.clone();
                    main_win.on_window_event(move |event| {
                        if let tauri::WindowEvent::Focused(focused) = event {
                            let app_handle = win_clone.app_handle();
                            let state = app_handle.state::<AppState>();
                            let effect = {
                                let config = state.config.lock().unwrap();
                                config.display.effect_type.clone()
                            };
                            if *focused {
                                crate::commands::apply_vibrancy(&win_clone, &effect);
                            } else {
                                let _ = window_vibrancy::clear_mica(&win_clone);
                                let _ = window_vibrancy::clear_acrylic(&win_clone);
                            }
                        }
                    });
                }

                // 初始状态：如果不处于游戏状态且开启了任务栏模式，则隐藏桌面主浮窗
                if !is_game && config.taskbar.enabled {
                    let _ = main_win.hide();
                } else {
                    let _ = main_win.show();
                }
            }

            // Initialize taskbar window
            if let Some(taskbar_win) = app.get_webview_window("taskbar") {
                if config.taskbar.enabled && !is_game {
                    #[cfg(target_os = "windows")]
                    {
                        if let Some(hwnd) = get_hwnd_from_window(&taskbar_win) {
                            let scale = taskbar_win.scale_factor().unwrap_or(1.0);
                            let p_width = (300.0 * scale).round() as i32;
                            let p_height = (36.0 * scale).round() as i32;
                            let p_offset_x = (config.taskbar.offset_x as f64 * scale).round() as i32;
                            let p_offset_y = (config.taskbar.offset_y as f64 * scale).round() as i32;
                            unsafe {
                                let _ = embed_in_taskbar(hwnd as _, p_width, p_height, &config.taskbar.align, p_offset_x, p_offset_y);
                            }
                        }
                    }
                    let _ = taskbar_win.show();
                } else {
                    let _ = taskbar_win.hide();
                }
            }

            // Create System Tray Menu
            let toggle_click_through_i = MenuItem::with_id(app, "toggle_click_through", "🔳 开启/关闭穿透", true, None::<&str>)?;
            
            // Submenu Layout
            let layout_horizontal_i = MenuItem::with_id(app, "layout_horizontal", "水平", true, None::<&str>)?;
            let layout_vertical_i = MenuItem::with_id(app, "layout_vertical", "垂直", true, None::<&str>)?;
            let layout_grid_i = MenuItem::with_id(app, "layout_grid", "网格", true, None::<&str>)?;
            let layout_submenu = SubmenuBuilder::new(app, "📐 布局")
                .item(&layout_horizontal_i)
                .item(&layout_vertical_i)
                .item(&layout_grid_i)
                .build()?;

            let settings_i = MenuItem::with_id(app, "settings", "⚙️ 设置...", true, None::<&str>)?;
            let toggle_visible_i = MenuItem::with_id(app, "toggle_visible", "👁 显示/隐藏窗口", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "❌ 退出", true, None::<&str>)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&toggle_click_through_i)
                .separator()
                .item(&layout_submenu)
                .separator()
                .item(&settings_i)
                .item(&toggle_visible_i)
                .separator()
                .item(&quit_i)
                .build()?;

            // Build Tray Icon
            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .icon(app.default_window_icon().unwrap().clone())
                .show_menu_on_left_click(false)
                .build(app)?;

            Ok(())
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = if window.is_visible().unwrap_or(true) {
                        window.hide()
                    } else {
                        window.show().and_then(|_| window.set_focus())
                    };
                }
            }
        })
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            match event.id().as_ref() {
                "quit" => app.exit(0),
                "settings" => {
                    let _ = crate::commands::open_settings_window(app.clone(), state.clone());
                }
                "toggle_visible" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = if window.is_visible().unwrap_or(true) {
                            window.hide()
                        } else {
                            window.show().and_then(|_| window.set_focus())
                        };
                    }
                }
                "toggle_click_through" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let mut config = state.config.lock().unwrap();
                        config.window.click_through = !config.window.click_through;
                        let _ = window.set_ignore_cursor_events(config.window.click_through);
                        let _ = crate::config::save_config(&config);
                        let _ = window.emit("config-changed", &*config);
                    }
                }
                "layout_horizontal" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let mut config = state.config.lock().unwrap();
                        config.display.layout = "horizontal".to_string();
                        let _ = crate::config::save_config(&config);
                        let _ = window.emit("config-changed", &*config);
                    }
                }
                "layout_vertical" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let mut config = state.config.lock().unwrap();
                        config.display.layout = "vertical".to_string();
                        let _ = crate::config::save_config(&config);
                        let _ = window.emit("config-changed", &*config);
                    }
                }
                "layout_grid" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let mut config = state.config.lock().unwrap();
                        config.display.layout = "grid".to_string();
                        let _ = crate::config::save_config(&config);
                        let _ = window.emit("config-changed", &*config);
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
