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

use crate::app_state::AppState;
use crate::config::{load_config, update_autostart_registry};

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
            commands::start_drag
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Setup background thread for metric collection
            let thread_app = app_handle.clone();
            thread::spawn(move || {
                let mut collector = collectors::SystemCollector::new();
                loop {
                    let state = thread_app.state::<AppState>();
                    let interval = {
                        let config = state.config.lock().unwrap();
                        config.update_interval_ms
                    };

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
                    if let Some(settings_win) = thread_app.get_webview_window("settings") {
                        let _ = settings_win.emit("metrics-update", &snapshot);
                    }

                    thread::sleep(Duration::from_millis(interval));
                }
            });

            // Initialize main window
            if let Some(main_win) = app.get_webview_window("main") {
                let state = app.state::<AppState>();
                let config = state.config.lock().unwrap().clone();
                
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
