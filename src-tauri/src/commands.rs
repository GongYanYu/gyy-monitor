use tauri::Manager;
use tauri::Emitter;
use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder, ContextMenu};
use crate::app_state::AppState;
use crate::config::{Config, save_config, update_autostart_registry};

// Recursively update nested JSON values by dot-separated path (accepted by borrow checker)
fn update_json_value(root: &mut serde_json::Value, parts: &[&str], value: serde_json::Value) {
    if parts.is_empty() {
        return;
    }
    if parts.len() == 1 {
        if let Some(obj) = root.as_object_mut() {
            obj.insert(parts[0].to_string(), value);
        }
        return;
    }
    if let Some(obj) = root.as_object_mut() {
        let next_entry = obj.entry(parts[0].to_string()).or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        update_json_value(next_entry, &parts[1..], value);
    }
}

pub fn apply_vibrancy(window: &tauri::WebviewWindow, effect_type: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = window_vibrancy::clear_mica(window);
        let _ = window_vibrancy::clear_acrylic(window);
        if window.is_focused().unwrap_or(true) {
            match effect_type {
                "mica" => {
                    let _ = window_vibrancy::apply_mica(window, Some(true));
                }
                "acrylic" => {
                    let _ = window_vibrancy::apply_acrylic(window, Some((20, 20, 20, 10)));
                }
                _ => {}
            }
        }
    }
}

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_config(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<bool, String> {
    let mut current_config = state.config.lock().unwrap();
    let mut config_val = serde_json::to_value(&*current_config).map_err(|e| e.to_string())?;
    
    let parts: Vec<&str> = key.split('.').collect();
    update_json_value(&mut config_val, &parts, value.clone());
    let new_config: Config = serde_json::from_value(config_val).map_err(|e| e.to_string())?;
    
    // Apply changes locally
    *current_config = new_config.clone();

    // Side-effects
    if key == "autostart" {
        if let Some(enabled) = value.as_bool() {
            let _ = update_autostart_registry(enabled);
        }
    } else if key == "window.always_on_top" {
        if let Some(always) = value.as_bool() {
            let _ = window.set_always_on_top(always);
        }
    } else if key == "window.click_through" {
        if let Some(through) = value.as_bool() {
            let _ = window.set_ignore_cursor_events(through);
        }
    } else if key == "display.effect_type" {
        if let Some(eff) = value.as_str() {
            if let Some(main_win) = window.get_webview_window("main") {
                apply_vibrancy(&main_win, eff);
            }
        }
    } else if key == "taskbar.enabled" {
        if let Some(enabled) = value.as_bool() {
            let is_game = {
                let state = window.state::<AppState>();
                state.is_game_active.load(std::sync::atomic::Ordering::Relaxed)
            };

            // Toggle main window visibility
            if let Some(main_win) = window.get_webview_window("main") {
                if is_game || !enabled {
                    let _ = main_win.show();
                } else {
                    let _ = main_win.hide();
                }
            }

            // Toggle taskbar window visibility
            if let Some(taskbar_win) = crate::get_taskbar_window(&window) {
                if enabled {
                    if !is_game {
                        #[cfg(target_os = "windows")]
                        {
                            if let Some(hwnd) = crate::get_hwnd_from_window(&taskbar_win) {
                                let scale = taskbar_win.scale_factor().unwrap_or(1.0);
                                let p_width = (300.0 * scale).round() as i32;
                                let p_height = (36.0 * scale).round() as i32;
                                let p_offset_x = (new_config.taskbar.offset_x as f64 * scale).round() as i32;
                                let p_offset_y = (new_config.taskbar.offset_y as f64 * scale).round() as i32;
                                unsafe {
                                    let _ = crate::embed_in_taskbar(hwnd as _, p_width, p_height, &new_config.taskbar.align, p_offset_x, p_offset_y);
                                }
                            }
                        }
                        let _ = taskbar_win.show();
                    }
                } else {
                    let _ = taskbar_win.hide();
                }
            }
        }
    } else if key == "taskbar.align" {
        if let Some(align_str) = value.as_str() {
            if let Some(taskbar_win) = crate::get_taskbar_window(&window) {
                #[cfg(target_os = "windows")]
                {
                    if let Some(hwnd) = crate::get_hwnd_from_window(&taskbar_win) {
                        if let Ok(size) = taskbar_win.outer_size() {
                            unsafe {
                                let scale = taskbar_win.scale_factor().unwrap_or(1.0);
                                let p_offset_x = (new_config.taskbar.offset_x as f64 * scale).round() as i32;
                                let p_offset_y = (new_config.taskbar.offset_y as f64 * scale).round() as i32;
                                crate::reposition_taskbar_window_hwnd(hwnd as _, size.width as i32, size.height as i32, align_str, p_offset_x, p_offset_y);
                            }
                        }
                    }
                }
            }
        }
    } else if key == "taskbar.offset_x" || key == "taskbar.offset_y" {
        if let Some(taskbar_win) = crate::get_taskbar_window(&window) {
            #[cfg(target_os = "windows")]
            {
                if let Some(hwnd) = crate::get_hwnd_from_window(&taskbar_win) {
                    if let Ok(size) = taskbar_win.outer_size() {
                        let scale = taskbar_win.scale_factor().unwrap_or(1.0);
                        let p_offset_x = (new_config.taskbar.offset_x as f64 * scale).round() as i32;
                        let p_offset_y = (new_config.taskbar.offset_y as f64 * scale).round() as i32;
                        unsafe {
                            crate::reposition_taskbar_window_hwnd(hwnd as _, size.width as i32, size.height as i32, &new_config.taskbar.align, p_offset_x, p_offset_y);
                        }
                    }
                }
            }
        }
    }

    // Save to file
    save_config(&current_config).map_err(|e| e.to_string())?;

    // Broadcast config update to all windows globally
    let _ = window.app_handle().emit("config-changed", &*current_config);

    Ok(true)
}

#[tauri::command]
pub fn set_config_bulk(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    patch: serde_json::Value,
) -> Result<bool, String> {
    let mut current_config = state.config.lock().unwrap();
    let mut config_val = serde_json::to_value(&*current_config).map_err(|e| e.to_string())?;

    // Recursively merge patch
    if let Some(patch_obj) = patch.as_object() {
        for (k, v) in patch_obj {
            if let Some(config_obj) = config_val.as_object_mut() {
                if config_obj.contains_key(k) && config_obj[k].is_object() && v.is_object() {
                    // Deep merge for one level nested objects (window, display)
                    if let (Some(c_nested), Some(p_nested)) = (config_obj[k].as_object_mut(), v.as_object()) {
                        for (nk, nv) in p_nested {
                            c_nested.insert(nk.to_string(), nv.clone());
                        }
                    }
                } else {
                    config_obj.insert(k.to_string(), v.clone());
                }
            }
        }
    }

    let new_config: Config = serde_json::from_value(config_val).map_err(|e| e.to_string())?;
    *current_config = new_config.clone();

    // Registry autostart
    let _ = update_autostart_registry(current_config.autostart);

    // Apply window settings
    let _ = window.set_always_on_top(current_config.window.always_on_top);
    let _ = window.set_ignore_cursor_events(current_config.window.click_through);

    // Apply vibrancy and visibility to main window
    let is_game = {
        let state = window.state::<AppState>();
        state.is_game_active.load(std::sync::atomic::Ordering::Relaxed)
    };
    if let Some(main_win) = window.get_webview_window("main") {
        apply_vibrancy(&main_win, &current_config.display.effect_type);
        if is_game || !current_config.taskbar.enabled {
            let _ = main_win.show();
        } else {
            let _ = main_win.hide();
        }
    }

    // Apply taskbar window visibility
    if let Some(taskbar_win) = crate::get_taskbar_window(&window) {
        if current_config.taskbar.enabled {
            let is_game = {
                let state = window.state::<AppState>();
                state.is_game_active.load(std::sync::atomic::Ordering::Relaxed)
            };
            if !is_game {
                #[cfg(target_os = "windows")]
                {
                    if let Some(hwnd) = crate::get_hwnd_from_window(&taskbar_win) {
                        let scale = taskbar_win.scale_factor().unwrap_or(1.0);
                        let p_width = (300.0 * scale).round() as i32;
                        let p_height = (36.0 * scale).round() as i32;
                        let p_offset_x = (current_config.taskbar.offset_x as f64 * scale).round() as i32;
                        let p_offset_y = (current_config.taskbar.offset_y as f64 * scale).round() as i32;
                        unsafe {
                            let _ = crate::embed_in_taskbar(hwnd as _, p_width, p_height, &current_config.taskbar.align, p_offset_x, p_offset_y);
                        }
                    }
                }
                let _ = taskbar_win.show();
            } else {
                let _ = taskbar_win.hide();
            }
        } else {
            let _ = taskbar_win.hide();
        }
    }

    // Save to file
    save_config(&current_config).map_err(|e| e.to_string())?;

    // Broadcast config update to all windows globally
    let _ = window.app_handle().emit("config-changed", &*current_config);

    Ok(true)
}

#[tauri::command]
pub fn apply_config_temp(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    patch: serde_json::Value,
) -> Result<bool, String> {
    let mut current_config = state.config.lock().unwrap();
    let mut config_val = serde_json::to_value(&*current_config).map_err(|e| e.to_string())?;

    if let Some(patch_obj) = patch.as_object() {
        for (k, v) in patch_obj {
            if let Some(config_obj) = config_val.as_object_mut() {
                if config_obj.contains_key(k) && config_obj[k].is_object() && v.is_object() {
                    if let (Some(c_nested), Some(p_nested)) = (config_obj[k].as_object_mut(), v.as_object()) {
                        for (nk, nv) in p_nested {
                            c_nested.insert(nk.to_string(), nv.clone());
                        }
                    }
                } else {
                    config_obj.insert(k.to_string(), v.clone());
                }
            }
        }
    }

    let new_config: Config = serde_json::from_value(config_val).map_err(|e| e.to_string())?;
    *current_config = new_config;

    // Temporary window style adjustments (no save)
    let _ = window.set_always_on_top(current_config.window.always_on_top);
    let _ = window.set_ignore_cursor_events(current_config.window.click_through);

    // Apply vibrancy to main window for preview
    if let Some(main_win) = window.get_webview_window("main") {
        apply_vibrancy(&main_win, &current_config.display.effect_type);
    }

    // Notify all windows to preview styles globally
    let _ = window.app_handle().emit("config-changed", &*current_config);

    Ok(true)
}

#[tauri::command]
pub fn resize_window_to_fit(window: tauri::Window, width: f64, height: f64) -> Result<bool, String> {
    if window.label().starts_with("taskbar") {
        #[cfg(target_os = "windows")]
        {
            if let Some(hwnd) = crate::get_hwnd_from_window(&window) {
                let scale_factor = window.scale_factor().unwrap_or(1.0);
                let physical_width = (width * scale_factor).round() as i32;
                let physical_height = (height * scale_factor).round() as i32;
                
                let state = window.state::<AppState>();
                let (align, offset_x, offset_y) = {
                    let config = state.config.lock().unwrap();
                    (config.taskbar.align.clone(), config.taskbar.offset_x, config.taskbar.offset_y)
                };
                let p_offset_x = (offset_x as f64 * scale_factor).round() as i32;
                let p_offset_y = (offset_y as f64 * scale_factor).round() as i32;
                unsafe {
                    crate::reposition_taskbar_window_hwnd(hwnd as _, physical_width, physical_height, &align, p_offset_x, p_offset_y);
                }
            }
        }
    } else {
        window.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height })).map_err(|e| e.to_string())?;
    }
    Ok(true)
}

#[tauri::command]
pub fn toggle_click_through(window: tauri::Window, enabled: bool) -> Result<bool, String> {
    window.set_ignore_cursor_events(enabled).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn move_window(window: tauri::Window, dx: i32, dy: i32) -> Result<bool, String> {
    if let Ok(pos) = window.outer_position() {
        let new_x = pos.x + dx;
        let new_y = pos.y + dy;
        window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x: new_x, y: new_y })).map_err(|e| e.to_string())?;
    }
    Ok(true)
}

#[tauri::command]
pub fn toggle_visible(window: tauri::Window) -> Result<bool, String> {
    if window.is_visible().unwrap_or(true) {
        window.hide().map_err(|e| e.to_string())?;
    } else {
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(true)
}

#[tauri::command]
pub fn open_config_file() -> Result<bool, String> {
    let path = crate::config::get_config_path();
    let path_str = path.to_string_lossy().to_string();
    let _ = std::process::Command::new("cmd")
        .args(&["/c", "start", "", &path_str])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn read_file(path: String) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[tauri::command]
pub fn open_settings_window(app_handle: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    if let Some(settings_win) = app_handle.get_webview_window("settings") {
        let _ = settings_win.show();
        let _ = settings_win.set_focus();
    } else {
        let always_on_top = {
            let config = state.config.lock().unwrap();
            config.window.always_on_top
        };

        let settings_win = tauri::WebviewWindowBuilder::new(
            &app_handle,
            "settings",
            tauri::WebviewUrl::App("settings.html".into())
        )
        .title("gyy-monitor 设置")
        .inner_size(550.0, 650.0)
        .resizable(true)
        .always_on_top(always_on_top)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

        let app_clone = app_handle.clone();
        settings_win.on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Rollback config when settings window is closed
                let state = app_clone.state::<AppState>();
                let mut current_config = state.config.lock().unwrap();
                let saved_config = crate::config::load_config();
                *current_config = saved_config.clone();
                // Notify all windows to revert styles globally
                let _ = app_clone.emit("config-changed", &saved_config);
                if let Some(main_win) = app_clone.get_webview_window("main") {
                    apply_vibrancy(&main_win, &saved_config.display.effect_type);
                }
            }
        });
    }
    Ok(true)
}

#[tauri::command]
pub fn close_settings_window(app_handle: tauri::AppHandle) -> Result<bool, String> {
    if let Some(settings_win) = app_handle.get_webview_window("settings") {
        let _ = settings_win.close();
    }
    Ok(true)
}

#[tauri::command]
pub fn quit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
pub fn show_context_menu(window: tauri::Window) -> Result<(), String> {
    let app_handle = window.app_handle();
    
    let toggle_click_through_i = MenuItem::with_id(app_handle, "toggle_click_through", "🔳 开启/关闭穿透", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    
    let layout_horizontal_i = MenuItem::with_id(app_handle, "layout_horizontal", "水平", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let layout_vertical_i = MenuItem::with_id(app_handle, "layout_vertical", "垂直", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let layout_grid_i = MenuItem::with_id(app_handle, "layout_grid", "网格", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    
    let layout_submenu = SubmenuBuilder::new(app_handle, "📐 布局")
        .item(&layout_horizontal_i)
        .item(&layout_vertical_i)
        .item(&layout_grid_i)
        .build()
        .map_err(|e| e.to_string())?;

    let settings_i = MenuItem::with_id(app_handle, "settings", "⚙️ 设置...", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let toggle_visible_i = MenuItem::with_id(app_handle, "toggle_visible", "👁 显示/隐藏窗口", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit_i = MenuItem::with_id(app_handle, "quit", "❌ 退出应用", true, None::<&str>)
        .map_err(|e| e.to_string())?;

    let menu = MenuBuilder::new(app_handle)
        .item(&toggle_click_through_i)
        .separator()
        .item(&layout_submenu)
        .separator()
        .item(&settings_i)
        .item(&toggle_visible_i)
        .separator()
        .item(&quit_i)
        .build()
        .map_err(|e| e.to_string())?;

    menu.popup(window).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn start_drag(window: tauri::Window) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_metrics(state: tauri::State<'_, AppState>) -> serde_json::Value {
    let metrics = state.metrics.lock().unwrap();
    metrics.clone()
}
