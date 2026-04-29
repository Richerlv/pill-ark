use tauri::{Manager, LogicalSize, PhysicalPosition};

mod core;
mod ports;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // 获取主显示器
            let monitors = app.available_monitors()?;
            let primary_monitor = monitors.iter()
                .find(|m| m.position().x >= 0 && m.position().y >= 0)
                .or_else(|| monitors.iter().min_by_key(|m| m.position().x + m.position().y));

            if let Some(monitor) = primary_monitor {
                let monitor_size = monitor.size();
                let monitor_scale = monitor.scale_factor();
                let screen_width = monitor_size.width as f64 / monitor_scale;

                // 基础胶囊宽度 185px，居中于刘海
                let notch_center_ratio = 0.522;
                let notch_center_x = screen_width * notch_center_ratio;
                let pill_width = 185.0;
                let x = (notch_center_x - pill_width / 2.0) as i32;

                eprintln!("DEBUG: screen_width={}, x={}", screen_width, x);
                window.set_position(PhysicalPosition::new(x * monitor_scale as i32, 0))?;
            }

            // 忽略点击事件，让刘海区域可穿透
            window.set_ignore_cursor_events(true)?;

            // 在 macOS 上设置窗口属性
            #[cfg(target_os = "macos")]
            {
                use objc2_app_kit::{NSWindow, NSWindowStyleMask, NSWindowCollectionBehavior};

                if let Ok(ns_window_ptr) = window.ns_window() {
                    unsafe {
                        let ns_window: &NSWindow = &*(ns_window_ptr as *const NSWindow);

                        // 设置 collectionBehavior
                        let behaviors = NSWindowCollectionBehavior(0x1 | 0x100 | 0x4);
                        ns_window.setCollectionBehavior(behaviors);

                        // 设置 level
                        ns_window.setLevel(25isize);

                        // 设置样式
                        let current_style = ns_window.styleMask();
                        ns_window.setStyleMask(current_style |
                            NSWindowStyleMask::FullSizeContentView |
                            NSWindowStyleMask::Borderless);
                        ns_window.setTitlebarAppearsTransparent(true);
                    }
                }
            }

            window.set_focus()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::get_running_tasks,
            core::get_task_count,
            set_window_size_and_center
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn set_window_size_and_center(window: tauri::Window, width: f64, height: f64) -> Result<(), String> {
    window.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;

    if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
        let monitor_size = monitor.size();
        let monitor_scale = monitor.scale_factor();
        let screen_width = monitor_size.width as f64 / monitor_scale;

        let notch_center_ratio = 0.522;
        let notch_center_x = screen_width * notch_center_ratio;
        let x = (notch_center_x - width / 2.0) as i32;
        window.set_position(PhysicalPosition::new(x * monitor_scale as i32, 0))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
