use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub pid: u32,
    pub status: TaskStatus,
    pub start_time: u64,
    pub command: String,
    pub tool: String,
    pub cwd: String,
    pub session_id: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Running,
    Idle,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeSession {
    pid: u32,
    session_id: String,
    cwd: String,
    started_at: u64,
    status: String,
    updated_at: u64,
}

fn claude_sessions_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".claude").join("sessions"))
}

fn is_active_claude_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "running" | "busy" | "working" | "thinking"
    )
}

fn task_status(status: &str) -> TaskStatus {
    match status.to_ascii_lowercase().as_str() {
        "running" | "busy" | "working" | "thinking" => TaskStatus::Running,
        "idle" => TaskStatus::Idle,
        "stopped" | "exited" | "completed" => TaskStatus::Stopped,
        _ => TaskStatus::Unknown,
    }
}

fn read_claude_sessions() -> Vec<ClaudeSession> {
    let Some(sessions_dir) = claude_sessions_dir() else {
        return Vec::new();
    };

    let Ok(entries) = fs::read_dir(sessions_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|content| serde_json::from_str::<ClaudeSession>(&content).ok())
        .collect()
}

#[tauri::command]
pub fn get_running_tasks() -> Vec<Task> {
    let mut tasks: Vec<Task> = read_claude_sessions()
        .into_iter()
        .filter(|session| is_active_claude_status(&session.status))
        .map(|session| {
            let project = PathBuf::from(&session.cwd)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| session.cwd.clone());

            Task {
                name: project,
                pid: session.pid,
                status: task_status(&session.status),
                start_time: session.started_at,
                command: format!("claude session {}", session.session_id),
                tool: "Claude Code".to_string(),
                cwd: session.cwd,
                session_id: session.session_id,
                updated_at: session.updated_at,
            }
        })
        .collect();

    tasks.sort_by_key(|task| task.updated_at);
    tasks
}

#[tauri::command]
pub fn get_task_count() -> usize {
    get_running_tasks().len()
}

#[tauri::command]
pub fn get_claude_sessions() -> Vec<Task> {
    let mut tasks: Vec<Task> = read_claude_sessions()
        .into_iter()
        .map(|session| {
            let project = PathBuf::from(&session.cwd)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| session.cwd.clone());

            Task {
                name: project,
                pid: session.pid,
                status: task_status(&session.status),
                start_time: session.started_at,
                command: format!("claude session {}", session.session_id),
                tool: "Claude Code".to_string(),
                cwd: session.cwd,
                session_id: session.session_id,
                updated_at: session.updated_at,
            }
        })
        .collect();

    tasks.sort_by_key(|task| task.updated_at);
    tasks
}

#[allow(dead_code)]
pub fn check_tasks_changed(last_tasks: &[String], current_tasks: &[Task]) -> bool {
    let last_names: Vec<String> = last_tasks.to_vec();
    let current_names: Vec<String> = current_tasks
        .iter()
        .map(|task| task.session_id.clone())
        .collect();

    last_names != current_names
}
