use tauri::{Manager, PhysicalPosition, LogicalSize};

mod core;
mod ports;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // 动态计算刘海位置
            // 先获取所有可用显示器，找主显示器（position.x >= 0 且最小）
            let monitors = app.available_monitors()?;
            eprintln!("DEBUG: found {} monitors", monitors.len());
            for (i, m) in monitors.iter().enumerate() {
                eprintln!("DEBUG: monitor {}: position={:?}, size={:?}, scale={}",
                    i, m.position(), m.size(), m.scale_factor());
            }

            // 主显示器通常是 position 最小的那个（或者 x=0 的）
            let primary_monitor = monitors.iter()
                .find(|m| m.position().x >= 0 && m.position().y >= 0)
                .or_else(|| monitors.iter().min_by_key(|m| m.position().x + m.position().y));

            if let Some(monitor) = primary_monitor {
                let monitor_size = monitor.size();
                let monitor_scale = monitor.scale_factor();

                eprintln!("DEBUG: selected monitor: size={:?}, scale={}", monitor_size, monitor_scale);

                // 刘海中心约在 x=785 logical
                // 胶囊宽度 120px，左边缘 = 785 - 60 = 725
                let target_x_logical = 725;
                let window_x_physical = (target_x_logical as f64 * monitor_scale) as i32;

                // 垂直位置：让窗口顶部对齐刘海上边缘（y=0），被刘海挡住也可以
                let window_y_physical = 0;

                eprintln!("DEBUG: window position: x={}, y={}", window_x_physical, window_y_physical);
                window.set_position(PhysicalPosition::new(window_x_physical, window_y_physical))?;
            } else {
                eprintln!("DEBUG: no monitor found, using fallback");
                window.set_position(PhysicalPosition::new(1450, 0))?;
            }

            window.set_focus()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::get_running_tasks,
            core::get_task_count,
            set_window_size
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn set_window_size(window: tauri::Window, width: f64, height: f64) -> Result<(), String> {
    window.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())
}