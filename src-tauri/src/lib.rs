use tauri::{Manager, PhysicalPosition};

mod core;
mod ports;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            window.set_focus()?;
            window.set_position(PhysicalPosition::new(1372, 0))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::get_running_tasks,
            core::get_task_count
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
