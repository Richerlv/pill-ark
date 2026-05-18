#[cfg(target_os = "macos")]
use objc2::MainThreadMarker;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSScreen;
use tauri::Manager;

mod core;
mod ports;

const INITIAL_WINDOW_WIDTH: f64 = 392.0;
const INITIAL_WINDOW_HEIGHT: f64 = 39.0;

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
                use objc2_foundation::{NSPoint, NSRect, NSSize};

                if let Ok(ns_window_ptr) = window.ns_window() {
                    unsafe {
                        let ns_window: &NSWindow = &*(ns_window_ptr as *const NSWindow);

                        // 使用主屏幕（带菜单栏的屏幕）
                        let mtm = MainThreadMarker::new().expect("Not on main thread");
                        let screen = NSScreen::mainScreen(mtm).expect("No main screen");
                        let screen_frame = screen.frame();

                        let window_width: f64 = INITIAL_WINDOW_WIDTH;
                        let window_height: f64 = INITIAL_WINDOW_HEIGHT;

                        // 使用完整屏幕宽度居中（不使用 safe area）
                        let x_pos =
                            screen_frame.origin.x + (screen_frame.size.width - window_width) / 2.0;
                        // Y坐标：屏幕物理最顶端
                        let y_pos =
                            screen_frame.origin.y + screen_frame.size.height - window_height;

                        // 设置窗口层级和行为，允许覆盖刘海区域
                        // NSWindowLevel::StatusBar = 25，菜单栏的级别
                        ns_window.setLevel(25);
                        ns_window.setCollectionBehavior(
                            NSWindowCollectionBehavior::CanJoinAllSpaces
                                | NSWindowCollectionBehavior::Stationary
                                | NSWindowCollectionBehavior::IgnoresCycle,
                        );

                        let new_frame = NSRect::new(
                            NSPoint::new(x_pos, y_pos),
                            NSSize::new(window_width, window_height),
                        );
                        ns_window.setFrame_display(new_frame, true);
                    }
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                position_webview_window_top_center(
                    &window,
                    INITIAL_WINDOW_WIDTH,
                    INITIAL_WINDOW_HEIGHT,
                )?;
            }

            window.set_ignore_cursor_events(false)?;

            window.set_focus()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core::get_running_tasks,
            core::get_task_count,
            core::get_claude_sessions,
            open_task_destination,
            set_window_size_and_center
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug)]
struct ProcessInfo {
    pid: u32,
    ppid: u32,
    command: String,
}

fn process_info(pid: u32) -> Option<ProcessInfo> {
    let output = std::process::Command::new("ps")
        .args([
            "-p",
            &pid.to_string(),
            "-o",
            "pid=",
            "-o",
            "ppid=",
            "-o",
            "comm=",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();
    let mut fields = line.split_whitespace();
    let pid = fields.next()?.parse().ok()?;
    let ppid = fields.next()?.parse().ok()?;
    let command = fields.collect::<Vec<_>>().join(" ");

    Some(ProcessInfo { pid, ppid, command })
}

#[cfg(target_os = "macos")]
fn activate_app_by_name(app_name: &str) -> Result<(), String> {
    std::process::Command::new("open")
        .args(["-a", app_name])
        .status()
        .map_err(|e| e.to_string())
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("failed to activate {app_name}"))
            }
        })
}

#[cfg(target_os = "macos")]
fn activate_terminal_window_for_tool(tool: &str) -> Result<(), String> {
    let script = r#"
on run argv
  set targetTool to item 1 of argv
  tell application "Terminal"
    repeat with w in windows
      set windowName to name of w as string

      if targetTool is "opencode" then
        if windowName contains "opencode" or windowName contains "OpenCode" or windowName contains "OC |" then
          set index of w to 1
          activate
          return "matched"
        end if
      end if

      if targetTool is "claude" then
        if (windowName contains "Claude Code" or windowName contains " claude ") and not (windowName contains "opencode" or windowName contains "OpenCode" or windowName contains "OC |") then
          set index of w to 1
          activate
          return "matched"
        end if
      end if
    end repeat
  end tell
  return "not matched"
end run
"#;
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .arg(tool)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() && String::from_utf8_lossy(&output.stdout).contains("matched") {
        Ok(())
    } else {
        Err("matching terminal window not found".to_string())
    }
}

#[cfg(target_os = "macos")]
fn activate_terminal_window_for_task(task: &core::Task) -> Result<(), String> {
    match task.tool.as_str() {
        "ClaudeCode" | "Claude Code" => activate_terminal_window_for_tool("claude"),
        "OpenCode" => activate_terminal_window_for_tool("opencode"),
        _ => Err("no terminal matcher for task tool".to_string()),
    }
}

#[cfg(target_os = "macos")]
fn host_app_name(command: &str) -> Option<&'static str> {
    let command = command.to_ascii_lowercase();

    if command.contains("terminal.app") {
        Some("Terminal")
    } else if command.contains("iterm") {
        Some("iTerm")
    } else if command.contains("visual studio code.app") || command.ends_with("/code") {
        Some("Visual Studio Code")
    } else if command.contains("cursor.app") || command.ends_with("/cursor") {
        Some("Cursor")
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn activate_host_from_process_tree(pid: u32) -> Result<(), String> {
    let mut current_pid = pid;

    for _ in 0..12 {
        let Some(process) = process_info(current_pid) else {
            break;
        };

        if let Some(app_name) = host_app_name(&process.command) {
            return activate_app_by_name(app_name);
        }

        if process.ppid == 0 || process.ppid == process.pid {
            break;
        }

        current_pid = process.ppid;
    }

    Err("host app not found".to_string())
}

#[cfg(target_os = "macos")]
fn activate_opencode_destination(task: &core::Task) -> Result<(), String> {
    activate_terminal_window_for_task(task).or_else(|_| activate_app_by_name("Terminal"))
}

fn open_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("task has no cwd".to_string());
    }

    #[cfg(target_os = "macos")]
    let command = ("open", vec![path]);

    #[cfg(target_os = "windows")]
    let command = ("explorer", vec![path]);

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let command = ("xdg-open", vec![path]);

    std::process::Command::new(command.0)
        .args(command.1)
        .status()
        .map_err(|e| e.to_string())
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("failed to open path {path}"))
            }
        })
}

#[tauri::command]
fn open_task_destination(task: core::Task) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if task.tool == "Codex" && activate_app_by_name("Codex").is_ok() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    if task.tool == "OpenCode" && activate_opencode_destination(&task).is_ok() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    if activate_terminal_window_for_task(&task).is_ok() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    if task.pid > 0 && activate_host_from_process_tree(task.pid).is_ok() {
        return Ok(());
    }

    open_path(&task.cwd)
}

#[cfg(not(target_os = "macos"))]
fn position_webview_window_top_center(
    window: &tauri::WebviewWindow,
    width: f64,
    height: f64,
) -> Result<(), String> {
    use tauri::{LogicalPosition, LogicalSize};

    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;

    if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
        let screen_size = monitor.size();
        let x_pos = (screen_size.width as f64 - width) / 2.0;
        let y_pos = 0.0;

        window
            .set_position(LogicalPosition::new(x_pos, y_pos))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn position_window_top_center(
    window: &tauri::Window,
    width: f64,
    height: f64,
) -> Result<(), String> {
    use tauri::{LogicalPosition, LogicalSize};

    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;

    if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
        let screen_size = monitor.size();
        let x_pos = (screen_size.width as f64 - width) / 2.0;
        let y_pos = 0.0;

        window
            .set_position(LogicalPosition::new(x_pos, y_pos))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn set_window_size_and_center(
    window: tauri::Window,
    width: f64,
    height: f64,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
        use objc2_foundation::{NSPoint, NSRect, NSSize};

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
                ns_window.setCollectionBehavior(
                    NSWindowCollectionBehavior::CanJoinAllSpaces
                        | NSWindowCollectionBehavior::Stationary
                        | NSWindowCollectionBehavior::IgnoresCycle,
                );

                let new_frame = NSRect::new(NSPoint::new(x_pos, y_pos), NSSize::new(width, height));
                ns_window.setFrame_display(new_frame, true);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        position_window_top_center(&window, width, height)?;
    }

    Ok(())
}
