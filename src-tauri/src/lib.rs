use tauri::{Manager, LogicalSize, PhysicalPosition};
use objc2_app_kit::NSScreen;
use objc2::MainThreadMarker;

mod core;
mod ports;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // 在 macOS 上使用安全区域定位
            #[cfg(target_os = "macos")]
            {
                use objc2_app_kit::NSWindow;

                if let Ok(ns_window_ptr) = window.ns_window() {
                    unsafe {
                        let _ns_window: &NSWindow = &*(ns_window_ptr as *const NSWindow);

                        // 使用主屏幕（带菜单栏的屏幕），而不是窗口当前所在的屏幕
                        let mtm = MainThreadMarker::new().expect("Not on main thread");
                        let screen = NSScreen::mainScreen(mtm).expect("No main screen");

                        let auxiliary_top_left = screen.auxiliaryTopLeftArea();
                        let auxiliary_top_right = screen.auxiliaryTopRightArea();
                        let screen_frame = screen.frame();

                        let window_width: f64 = 185.0;
                        let window_height: f64 = 37.0;

                        // X：使用 native API 计算刘海中心对齐
                        let aux_left_width = auxiliary_top_left.size.width;
                        let aux_right_width = auxiliary_top_right.size.width;
                        let screen_width = screen_frame.size.width;
                        let x_pos = aux_left_width + (screen_width - aux_left_width - aux_right_width - window_width) / 2.0;

                        // Y：窗口顶部在屏幕的最上方（macOS 坐标原点是左下角）
                        let y_pos = screen_frame.size.height - window_height;

                        // 使用 Tauri 设置位置
                        let _ = window.set_position(PhysicalPosition::new(x_pos as i32, y_pos as i32));
                        let _ = window.set_size(LogicalSize::new(window_width, window_height));
                    }
                }
            }

            // 忽略点击事件
            window.set_ignore_cursor_events(true)?;

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

    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWindow;
        use objc2_foundation::{NSPoint, NSSize, NSRect};

        if let Ok(ns_window_ptr) = window.ns_window() {
            unsafe {
                let ns_window: &NSWindow = &*(ns_window_ptr as *const NSWindow);

                let mtm = MainThreadMarker::new().expect("Not on main thread");
                let screen = NSScreen::mainScreen(mtm).expect("No main screen");
                let screen_frame = screen.frame();

                let y_pos = screen_frame.size.height - height;

                let auxiliary_top_left = screen.auxiliaryTopLeftArea();
                let auxiliary_top_right = screen.auxiliaryTopRightArea();
                let aux_left_width = auxiliary_top_left.size.width;
                let aux_right_width = auxiliary_top_right.size.width;
                let screen_width = screen_frame.size.width;
                let x_pos = aux_left_width + (screen_width - aux_left_width - aux_right_width - width) / 2.0;

                let new_frame = NSRect::new(
                    NSPoint::new(x_pos, y_pos),
                    NSSize::new(width, height)
                );
                ns_window.setFrame_display(new_frame, true);
            }
        }
    }

    Ok(())
}
