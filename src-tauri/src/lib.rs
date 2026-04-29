use tauri::{Manager, PhysicalPosition, LogicalSize};

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
                // logical 屏幕宽度
                let screen_width = monitor_size.width as f64 / monitor_scale;

                // 基础胶囊宽度 185px，居中于刘海
                // 根据实测调整比例，让胶囊中心对齐刘海中心
                let notch_center_ratio = 0.522;
                let notch_center_x = screen_width * notch_center_ratio;
                let pill_width = 185.0;
                let x = (notch_center_x - pill_width / 2.0) as i32;

                eprintln!("DEBUG: screen_width={}, pill_width={}, x={}", screen_width, pill_width, x);
                window.set_position(PhysicalPosition::new(x * monitor_scale as i32, 0))?;
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
    // 设置窗口大小
    window.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;

    // 重新计算居中位置（基于刘海中心比例）
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