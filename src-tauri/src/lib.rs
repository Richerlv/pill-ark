use tauri::Manager;
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

            // 在 macOS 上使用物理屏幕定位
            #[cfg(target_os = "macos")]
            {
                use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
                use objc2_foundation::{NSPoint, NSSize, NSRect};

                if let Ok(ns_window_ptr) = window.ns_window() {
                    unsafe {
                        let ns_window: &NSWindow = &*(ns_window_ptr as *const NSWindow);

                        // 使用主屏幕（带菜单栏的屏幕）
                        let mtm = MainThreadMarker::new().expect("Not on main thread");
                        let screen = NSScreen::mainScreen(mtm).expect("No main screen");
                        let screen_frame = screen.frame();

                        let window_width: f64 = 185.0;
                        let window_height: f64 = 37.0;

                        // 使用完整屏幕宽度居中（不使用 safe area）
                        let x_pos = screen_frame.origin.x + (screen_frame.size.width - window_width) / 2.0;
                        // Y坐标：屏幕物理最顶端
                        let y_pos = screen_frame.origin.y + screen_frame.size.height - window_height;

                        // 设置窗口层级和行为，允许覆盖刘海区域
                        // NSWindowLevel::StatusBar = 25，菜单栏的级别
                        ns_window.setLevel(25);
                        ns_window.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Stationary | NSWindowCollectionBehavior::IgnoresCycle);

                        let new_frame = NSRect::new(
                            NSPoint::new(x_pos, y_pos),
                            NSSize::new(window_width, window_height)
                        );
                        ns_window.setFrame_display(new_frame, true);
                    }
                }
            }

            window.set_ignore_cursor_events(false)?;

            window.set_focus()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::get_running_tasks,
            core::get_task_count,
            core::get_claude_sessions,
            set_window_size_and_center
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn set_window_size_and_center(window: tauri::Window, width: f64, height: f64) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
        use objc2_foundation::{NSPoint, NSSize, NSRect};

        if let Ok(ns_window_ptr) = window.ns_window() {
            unsafe {
                let ns_window: &NSWindow = &*(ns_window_ptr as *const NSWindow);

                let mtm = MainThreadMarker::new().expect("Not on main thread");
                let screen = NSScreen::mainScreen(mtm).expect("No main screen");
                let screen_frame = screen.frame();

                // 使用完整屏幕宽度居中（不使用 safe area）
                let x_pos = screen_frame.origin.x + (screen_frame.size.width - width) / 2.0;
                // Y坐标：屏幕物理最顶端
                let y_pos = screen_frame.origin.y + screen_frame.size.height - height;

                // 设置窗口层级和行为，允许覆盖刘海区域
                // NSWindowLevel::StatusBar = 25，菜单栏的级别
                ns_window.setLevel(25);
                ns_window.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Stationary | NSWindowCollectionBehavior::IgnoresCycle);

                let new_frame = NSRect::new(
                    NSPoint::new(x_pos, y_pos),
                    NSSize::new(width, height)
                );
                ns_window.setFrame_display(new_frame, true);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        window.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
    }

    Ok(())
}
