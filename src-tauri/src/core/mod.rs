use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

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

fn opencode_data_dir() -> Option<PathBuf> {
    env::var_os("OPENCODE_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local").join("share").join("opencode"))
        })
}

fn is_active_agent_status(status: &str) -> bool {
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

fn project_name(cwd: &str) -> String {
    PathBuf::from(cwd)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| cwd.to_string())
}

fn claude_tasks(include_inactive: bool) -> Vec<Task> {
    read_claude_sessions()
        .into_iter()
        .filter(|session| include_inactive || is_active_agent_status(&session.status))
        .map(|session| {
            let project = project_name(&session.cwd);

            Task {
                name: project,
                pid: session.pid,
                status: task_status(&session.status),
                start_time: session.started_at,
                command: format!("claude session {}", session.session_id),
                tool: "ClaudeCode".to_string(),
                cwd: session.cwd,
                session_id: format!("claude:{}", session.session_id),
                updated_at: session.updated_at,
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
struct OpenCodeSession {
    id: String,
    directory: String,
    title: String,
    time_updated: u64,
}

#[derive(Debug, Clone)]
struct OpenCodeActiveMessage {
    id: String,
    session_id: String,
    time_created: u64,
    time_updated: u64,
}

#[derive(Debug, Clone)]
struct OpenCodePromptActivity {
    session_id: String,
    updated_at: u64,
}

fn sqlite3_bin() -> &'static str {
    if Path::new("/usr/bin/sqlite3").exists() {
        "/usr/bin/sqlite3"
    } else {
        "sqlite3"
    }
}

fn read_opencode_sessions() -> Vec<OpenCodeSession> {
    let Some(data_dir) = opencode_data_dir() else {
        return Vec::new();
    };
    let db_path = data_dir.join("opencode.db");
    if !db_path.exists() {
        return Vec::new();
    }

    let query = "select id, directory, title, time_updated from session where time_archived is null order by time_updated desc;";
    let Ok(output) = Command::new(sqlite3_bin())
        .args(["-readonly", "-separator", "\u{1f}"])
        .arg(db_path)
        .arg(query)
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\u{1f}');
            let id = fields.next()?.to_string();
            let directory = fields.next()?.to_string();
            let title = fields.next()?.to_string();
            let time_updated = fields.next()?.parse().ok()?;

            Some(OpenCodeSession {
                id,
                directory,
                title,
                time_updated,
            })
        })
        .collect()
}

fn opencode_log_dir() -> Option<PathBuf> {
    opencode_data_dir().map(|data_dir| data_dir.join("log"))
}

fn unix_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn modified_time_millis(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn assistant_message_is_active(data: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(data) else {
        return false;
    };

    let is_assistant = value
        .get("role")
        .and_then(|role| role.as_str())
        .is_some_and(|role| role == "assistant");
    if !is_assistant {
        return false;
    }

    let has_completed_at = value
        .get("time")
        .and_then(|time| time.get("completed"))
        .is_some();
    let has_finish_reason = value.get("finish").is_some();

    !has_completed_at && !has_finish_reason
}

fn read_opencode_latest_messages() -> HashMap<String, OpenCodeActiveMessage> {
    let Some(data_dir) = opencode_data_dir() else {
        return HashMap::new();
    };
    let db_path = data_dir.join("opencode.db");
    if !db_path.exists() {
        return HashMap::new();
    }

    let query = "select id, session_id, time_created, time_updated, data from message order by time_updated desc limit 200;";
    let Ok(output) = Command::new(sqlite3_bin())
        .args(["-readonly", "-separator", "\u{1f}"])
        .arg(db_path)
        .arg(query)
        .output()
    else {
        return HashMap::new();
    };

    if !output.status.success() {
        return HashMap::new();
    }

    let mut seen_sessions = HashSet::new();

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(5, '\u{1f}');
            let id = fields.next()?.to_string();
            let session_id = fields.next()?.to_string();
            let time_created = fields.next()?.parse().ok()?;
            let time_updated = fields.next()?.parse().ok()?;
            let data = fields.next()?;

            if !assistant_message_is_active(data) || !seen_sessions.insert(session_id.clone()) {
                return None;
            }

            Some(OpenCodeActiveMessage {
                id,
                session_id,
                time_created,
                time_updated,
            })
        })
        .map(|message| (message.session_id.clone(), message))
        .collect()
}

fn line_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    Some(&rest[..end])
}

fn line_session_id(line: &str) -> Option<String> {
    let session_id = line_value(line, "sessionID=").or_else(|| line_value(line, "session.id="))?;

    if session_id.is_empty() {
        None
    } else {
        Some(session_id.to_string())
    }
}

fn opencode_prompt_activity_from_logs(
    logs: impl IntoIterator<Item = (u64, String)>,
) -> Vec<OpenCodePromptActivity> {
    let mut active_sessions = HashMap::new();

    for (modified_at, content) in logs {
        for line in content.lines() {
            let has_session_id = line.contains("sessionID=") || line.contains("session.id=");
            let starts_prompt = line.contains("service=session.prompt")
                && has_session_id
                && ((line.contains(" loop") && !line.contains(" exiting loop"))
                    || line.contains("status=started"));
            let starts_stream =
                line.contains("service=llm") && has_session_id && line.contains(" stream");
            let stops_prompt = line.contains("service=session.prompt")
                && has_session_id
                && (line.contains(" exiting loop") || line.contains(" cancel"));
            let worker_stops = line.contains("service=default worker shutting down");

            if worker_stops {
                active_sessions.clear();
                continue;
            }

            if starts_prompt || starts_stream {
                if let Some(session_id) = line_session_id(line) {
                    active_sessions.insert(session_id, modified_at);
                }
                continue;
            }

            if stops_prompt {
                if let Some(session_id) = line_session_id(line) {
                    active_sessions.remove(&session_id);
                }
            }
        }
    }

    active_sessions
        .into_iter()
        .map(|(session_id, updated_at)| OpenCodePromptActivity {
            session_id,
            updated_at,
        })
        .collect()
}

fn read_opencode_prompt_activity() -> Vec<OpenCodePromptActivity> {
    let Some(log_dir) = opencode_log_dir() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(log_dir) else {
        return Vec::new();
    };

    let now = unix_time_millis();
    let mut logs = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "log"))
        .filter_map(|path| modified_time_millis(&path).map(|modified_at| (path, modified_at)))
        .filter(|(_, modified_at)| now.saturating_sub(*modified_at) <= 30 * 60 * 1000)
        .collect::<Vec<_>>();

    logs.sort_by_key(|(_, modified_at)| *modified_at);

    opencode_prompt_activity_from_logs(logs.into_iter().filter_map(|(path, modified_at)| {
        fs::read_to_string(path)
            .ok()
            .map(|content| (modified_at, content))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_sessions(log: &str) -> Vec<String> {
        let mut sessions = opencode_prompt_activity_from_logs([(1, log.to_string())])
            .into_iter()
            .map(|activity| activity.session_id)
            .collect::<Vec<_>>();
        sessions.sort();
        sessions
    }

    #[test]
    fn opencode_prompt_start_marks_session_active() {
        let sessions = active_sessions(
            "INFO service=session.prompt step=0 sessionID=ses_running loop\n\
             INFO service=llm providerID=opencode sessionID=ses_running stream",
        );

        assert_eq!(sessions, vec!["ses_running"]);
    }

    #[test]
    fn opencode_prompt_start_accepts_session_dot_id() {
        let sessions = active_sessions(
            "INFO service=session.prompt session.id=ses_running step=0 loop\n\
             INFO service=llm providerID=opencode session.id=ses_running stream",
        );

        assert_eq!(sessions, vec!["ses_running"]);
    }

    #[test]
    fn opencode_prompt_exit_clears_session() {
        let sessions = active_sessions(
            "INFO service=session.prompt step=0 sessionID=ses_done loop\n\
             INFO service=llm providerID=opencode sessionID=ses_done stream\n\
             INFO service=session.prompt sessionID=ses_done exiting loop",
        );

        assert!(sessions.is_empty());
    }

    #[test]
    fn opencode_prompt_exit_accepts_session_dot_id() {
        let sessions = active_sessions(
            "INFO service=session.prompt session.id=ses_done step=0 loop\n\
             INFO service=llm providerID=opencode session.id=ses_done stream\n\
             INFO service=session.prompt session.id=ses_done exiting loop",
        );

        assert!(sessions.is_empty());
    }

    #[test]
    fn opencode_worker_shutdown_clears_stale_prompt() {
        let sessions = active_sessions(
            "INFO service=session.prompt step=0 sessionID=ses_stale loop\n\
             INFO service=llm providerID=opencode sessionID=ses_stale stream\n\
             INFO service=default worker shutting down",
        );

        assert!(sessions.is_empty());
    }

    #[test]
    fn opencode_idle_startup_log_is_not_active() {
        let sessions = active_sessions(
            "INFO service=default directory=/tmp creating instance\n\
             INFO service=bus type=message.updated subscribing\n\
             INFO service=provider status=completed duration=60 state",
        );

        assert!(sessions.is_empty());
    }
}

fn opencode_tasks() -> Vec<Task> {
    let sessions = read_opencode_sessions();
    let latest_messages = read_opencode_latest_messages();

    read_opencode_prompt_activity()
        .into_iter()
        .filter_map(|activity| {
            let session = sessions
                .iter()
                .find(|session| session.id == activity.session_id)?;
            let message = latest_messages.get(&session.id);
            let name = Some(session.title.clone())
                .filter(|title| !title.is_empty())
                .unwrap_or_else(|| project_name(&session.directory));

            Some(Task {
                name,
                pid: 0,
                status: TaskStatus::Running,
                start_time: message
                    .map(|message| message.time_created)
                    .unwrap_or(session.time_updated),
                command: format!("opencode session {}", session.id),
                tool: "OpenCode".to_string(),
                cwd: session.directory.clone(),
                session_id: format!(
                    "opencode:{}:{}",
                    session.id,
                    message
                        .map(|message| message.id.as_str())
                        .unwrap_or("prompt")
                ),
                updated_at: message
                    .map(|message| message.time_updated)
                    .unwrap_or(activity.updated_at)
                    .max(session.time_updated),
            })
        })
        .collect()
}

#[tauri::command]
pub fn get_running_tasks() -> Vec<Task> {
    let mut tasks = claude_tasks(false);
    tasks.extend(opencode_tasks());
    tasks.sort_by_key(|task| task.updated_at);
    tasks
}

#[tauri::command]
pub fn get_task_count() -> usize {
    get_running_tasks().len()
}

#[tauri::command]
pub fn get_claude_sessions() -> Vec<Task> {
    let mut tasks = claude_tasks(true);
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
