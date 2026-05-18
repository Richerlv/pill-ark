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

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            let mut home = PathBuf::from(drive);
            home.push(path);
            Some(home)
        })
}

fn claude_sessions_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".claude").join("sessions"))
}

fn opencode_data_dir() -> Option<PathBuf> {
    env::var_os("OPENCODE_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|local_app_data| local_app_data.join("opencode"))
                .filter(|path| path.exists())
        })
        .or_else(|| {
            env::var_os("APPDATA")
                .map(PathBuf::from)
                .map(|app_data| app_data.join("opencode"))
                .filter(|path| path.exists())
        })
        .or_else(|| {
            home_dir()
                .map(|home| home.join(".local").join("share").join("opencode"))
                .filter(|path| path.exists())
        })
        .or_else(|| home_dir().map(|home| home.join(".local").join("share").join("opencode")))
}

fn codex_home_dir() -> Option<PathBuf> {
    env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".codex")))
}

fn codex_logs_db_path() -> Option<PathBuf> {
    codex_home_dir()
        .map(|home| home.join("logs_2.sqlite"))
        .filter(|path| path.exists())
}

fn codex_state_db_path() -> Option<PathBuf> {
    codex_home_dir()
        .map(|home| home.join("state_5.sqlite"))
        .filter(|path| path.exists())
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

fn clean_prompt_text(text: &str) -> Option<String> {
    let prompt = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if prompt.is_empty() {
        None
    } else {
        Some(prompt)
    }
}

fn claude_project_dirs() -> Vec<PathBuf> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    let projects_dir = home.join(".claude").join("projects");
    let Ok(entries) = fs::read_dir(projects_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

fn claude_prompt_from_content(content: &serde_json::Value) -> Option<String> {
    content.as_str().and_then(clean_prompt_text).or_else(|| {
        content.as_array().and_then(|parts| {
            clean_prompt_text(
                &parts
                    .iter()
                    .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        })
    })
}

fn read_claude_last_prompt(session_id: &str) -> Option<String> {
    for project_dir in claude_project_dirs() {
        let transcript_path = project_dir.join(format!("{session_id}.jsonl"));
        if !transcript_path.exists() {
            continue;
        }

        let Ok(content) = fs::read_to_string(transcript_path) else {
            continue;
        };

        for line in content.lines().rev() {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };

            if let Some(prompt) = value
                .get("lastPrompt")
                .and_then(|prompt| prompt.as_str())
                .and_then(clean_prompt_text)
            {
                return Some(prompt);
            }

            let is_user = value
                .get("type")
                .and_then(|message_type| message_type.as_str())
                .is_some_and(|message_type| message_type == "user");
            if !is_user {
                continue;
            }

            if let Some(prompt) = value
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(claude_prompt_from_content)
            {
                return Some(prompt);
            }
        }
    }

    None
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
            let name = read_claude_last_prompt(&session.session_id).unwrap_or(project);

            Task {
                name,
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
    prompt: Option<String>,
}

#[derive(Debug, Clone)]
struct OpenCodePromptActivity {
    session_id: String,
    updated_at: u64,
}

#[derive(Debug, Clone)]
struct CodexThread {
    id: String,
    title: String,
    cwd: String,
    updated_at: u64,
    first_user_message: String,
    preview: String,
}

#[derive(Debug, Clone)]
struct CodexTurnActivity {
    thread_id: String,
    turn_id: String,
    start_time: u64,
    updated_at: u64,
    cwd: Option<String>,
    completed: bool,
}

fn sqlite3_bin() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        return "sqlite3.exe";
    }

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

fn active_assistant_parent_id(data: &str) -> Option<Option<String>> {
    let value = serde_json::from_str::<serde_json::Value>(data).ok()?;
    let is_assistant = value
        .get("role")
        .and_then(|role| role.as_str())
        .is_some_and(|role| role == "assistant");
    if !is_assistant {
        return None;
    }

    let has_completed_at = value
        .get("time")
        .and_then(|time| time.get("completed"))
        .is_some();
    let has_finish_reason = value.get("finish").is_some();

    if has_completed_at || has_finish_reason {
        return None;
    }

    Some(
        value
            .get("parentID")
            .and_then(|parent_id| parent_id.as_str())
            .map(|parent_id| parent_id.to_string()),
    )
}

fn user_message_role(data: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(data)
        .ok()
        .and_then(|value| {
            value
                .get("role")
                .and_then(|role| role.as_str())
                .map(|role| role == "user")
        })
        .unwrap_or(false)
}

fn opencode_part_text(data: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(data).ok()?;
    let is_text = value
        .get("type")
        .and_then(|part_type| part_type.as_str())
        .is_some_and(|part_type| part_type == "text");
    if !is_text {
        return None;
    }

    value
        .get("text")
        .and_then(|text| text.as_str())
        .and_then(clean_prompt_text)
}

#[derive(Debug, Clone)]
struct OpenCodeUserPrompt {
    session_id: String,
    time_created: u64,
    text: String,
}

fn read_opencode_user_prompts(db_path: &Path) -> HashMap<String, OpenCodeUserPrompt> {
    let query = "select m.id, m.session_id, m.time_created, m.data, p.data from message m join part p on p.message_id = m.id order by m.time_created desc, p.time_created asc limit 1000;";
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

    let mut prompts: HashMap<String, OpenCodeUserPrompt> = HashMap::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut fields = line.splitn(5, '\u{1f}');
        let Some(id) = fields.next().map(str::to_string) else {
            continue;
        };
        let Some(session_id) = fields.next().map(str::to_string) else {
            continue;
        };
        let Some(time_created) = fields.next().and_then(|time| time.parse().ok()) else {
            continue;
        };
        let Some(message_data) = fields.next() else {
            continue;
        };
        let Some(part_data) = fields.next() else {
            continue;
        };

        if !user_message_role(message_data) {
            continue;
        }

        let Some(text) = opencode_part_text(part_data) else {
            continue;
        };

        prompts
            .entry(id)
            .and_modify(|prompt| {
                prompt.text.push(' ');
                prompt.text.push_str(&text);
            })
            .or_insert(OpenCodeUserPrompt {
                session_id,
                time_created,
                text,
            });
    }

    prompts
}

fn read_opencode_latest_messages() -> HashMap<String, OpenCodeActiveMessage> {
    let Some(data_dir) = opencode_data_dir() else {
        return HashMap::new();
    };
    let db_path = data_dir.join("opencode.db");
    if !db_path.exists() {
        return HashMap::new();
    }

    let user_prompts = read_opencode_user_prompts(&db_path);
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
            let parent_id = active_assistant_parent_id(data)?;

            if !seen_sessions.insert(session_id.clone()) {
                return None;
            }

            let prompt = parent_id
                .as_ref()
                .and_then(|parent_id| user_prompts.get(parent_id))
                .map(|prompt| prompt.text.clone())
                .or_else(|| {
                    user_prompts
                        .values()
                        .filter(|prompt| {
                            prompt.session_id == session_id && prompt.time_created <= time_created
                        })
                        .max_by_key(|prompt| prompt.time_created)
                        .map(|prompt| prompt.text.clone())
                });

            Some(OpenCodeActiveMessage {
                id,
                session_id,
                time_created,
                time_updated,
                prompt,
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

fn telemetry_value(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest
        .find(|ch: char| ch.is_whitespace() || matches!(ch, '}' | ']' | ')' | ',' | ';'))
        .unwrap_or(rest.len());
    Some(&rest[..end])
        .map(|value| {
            value
                .trim_matches(|ch| matches!(ch, '"' | '\'' | ',' | ';' | ':' | '}' | ']' | ')'))
                .to_string()
        })
        .filter(|value| !value.is_empty())
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

fn sqlite_field(value: &str) -> String {
    value.replace('\u{1f}', " ").trim().to_string()
}

fn read_codex_threads() -> HashMap<String, CodexThread> {
    let Some(db_path) = codex_state_db_path() else {
        return HashMap::new();
    };

    let clean = |column: &str| {
        format!("replace(replace(coalesce({column},''), char(10), ' '), char(31), ' ')")
    };
    let query = format!(
        "select id, {title}, {cwd}, coalesce(updated_at_ms, updated_at * 1000, 0), {first_user_message}, {preview} from threads order by coalesce(updated_at_ms, updated_at * 1000, 0) desc limit 200;",
        title = clean("title"),
        cwd = clean("cwd"),
        first_user_message = clean("first_user_message"),
        preview = clean("preview")
    );
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

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(6, '\u{1f}');
            let id = sqlite_field(fields.next()?);
            let title = sqlite_field(fields.next()?);
            let cwd = sqlite_field(fields.next()?);
            let updated_at = fields.next()?.trim().parse().ok()?;
            let first_user_message = sqlite_field(fields.next()?);
            let preview = sqlite_field(fields.next()?);

            Some((
                id.clone(),
                CodexThread {
                    id,
                    title,
                    cwd,
                    updated_at,
                    first_user_message,
                    preview,
                },
            ))
        })
        .collect()
}

fn codex_turn_completed(line: &str) -> bool {
    line.contains("event.kind=response.completed")
        || line.contains("\"type\":\"response.completed\"")
        || line.contains("response.completed")
}

fn codex_turn_activity_from_log_rows(
    rows: impl IntoIterator<Item = (u64, String, String)>,
) -> HashMap<String, CodexTurnActivity> {
    let mut turns = HashMap::new();

    for (timestamp, thread_id, body) in rows {
        let Some(turn_id) =
            telemetry_value(&body, "turn.id=").or_else(|| telemetry_value(&body, "turn_id="))
        else {
            continue;
        };
        let thread_id = if thread_id.is_empty() {
            telemetry_value(&body, "thread.id=").unwrap_or_default()
        } else {
            thread_id
        };
        if thread_id.is_empty() {
            continue;
        }

        let key = format!("{thread_id}:{turn_id}");
        let is_completed = codex_turn_completed(&body);
        let cwd = telemetry_value(&body, "cwd=");

        turns
            .entry(key)
            .and_modify(|activity: &mut CodexTurnActivity| {
                activity.updated_at = activity.updated_at.max(timestamp);
                activity.start_time = activity.start_time.min(timestamp);
                activity.completed |= is_completed;
                if activity.cwd.is_none() {
                    activity.cwd = cwd.clone();
                }
            })
            .or_insert(CodexTurnActivity {
                thread_id,
                turn_id,
                start_time: timestamp,
                updated_at: timestamp,
                cwd,
                completed: is_completed,
            });
    }

    turns
}

fn read_codex_turn_activity() -> HashMap<String, CodexTurnActivity> {
    let Some(db_path) = codex_logs_db_path() else {
        return HashMap::new();
    };

    let query = "select (ts * 1000 + ts_nanos / 1000000), coalesce(thread_id,''), replace(replace(coalesce(feedback_log_body,''), char(10), ' '), char(31), ' ') from logs where ts >= strftime('%s','now') - 1800 and feedback_log_body like '%session_task.turn%' order by ts desc, ts_nanos desc, id desc limit 4000;";
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

    codex_turn_activity_from_log_rows(String::from_utf8_lossy(&output.stdout).lines().filter_map(
        |line| {
            let mut fields = line.splitn(3, '\u{1f}');
            let timestamp = fields.next()?.parse().ok()?;
            let thread_id = fields.next()?.to_string();
            let body = fields.next()?.to_string();

            Some((timestamp, thread_id, body))
        },
    ))
}

fn codex_thread_name(thread: &CodexThread) -> String {
    [&thread.preview, &thread.first_user_message, &thread.title]
        .into_iter()
        .find_map(|value| clean_prompt_text(value))
        .unwrap_or_else(|| {
            if thread.cwd.is_empty() {
                thread.id.clone()
            } else {
                project_name(&thread.cwd)
            }
        })
}

fn codex_tasks() -> Vec<Task> {
    const CODEX_STALE_AFTER_MS: u64 = 5 * 60 * 1000;

    let now = unix_time_millis();
    let threads = read_codex_threads();

    read_codex_turn_activity()
        .into_values()
        .filter(|activity| {
            !activity.completed && now.saturating_sub(activity.updated_at) <= CODEX_STALE_AFTER_MS
        })
        .filter_map(|activity| {
            let thread = threads.get(&activity.thread_id);
            let cwd = activity
                .cwd
                .clone()
                .or_else(|| thread.map(|thread| thread.cwd.clone()))
                .unwrap_or_default();
            let name = thread
                .map(codex_thread_name)
                .or_else(|| {
                    if cwd.is_empty() {
                        None
                    } else {
                        Some(project_name(&cwd))
                    }
                })
                .unwrap_or_else(|| activity.thread_id.clone());

            Some(Task {
                name,
                pid: 0,
                status: TaskStatus::Running,
                start_time: activity.start_time,
                command: format!("codex turn {}", activity.turn_id),
                tool: "Codex".to_string(),
                cwd,
                session_id: format!("codex:{}:{}", activity.thread_id, activity.turn_id),
                updated_at: activity
                    .updated_at
                    .max(thread.map(|thread| thread.updated_at).unwrap_or(0)),
            })
        })
        .collect()
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

    #[test]
    fn prompt_text_is_cleaned_for_display() {
        assert_eq!(
            clean_prompt_text("  build   a nicer\n\n island  "),
            Some("build a nicer island".to_string())
        );
    }

    #[test]
    fn opencode_active_assistant_exposes_parent_message() {
        let parent_id = active_assistant_parent_id(
            r#"{"parentID":"msg_user","role":"assistant","time":{"created":1}}"#,
        );

        assert_eq!(parent_id, Some(Some("msg_user".to_string())));
    }

    #[test]
    fn opencode_completed_assistant_is_not_active() {
        let parent_id = active_assistant_parent_id(
            r#"{"parentID":"msg_user","role":"assistant","time":{"created":1,"completed":2},"finish":"stop"}"#,
        );

        assert_eq!(parent_id, None);
    }

    #[test]
    fn opencode_text_part_extracts_prompt_text() {
        assert_eq!(
            opencode_part_text(r#"{"type":"text","text":"  replace new session  "}"#),
            Some("replace new session".to_string())
        );
        assert_eq!(
            opencode_part_text(r#"{"type":"reasoning","text":"hidden"}"#),
            None
        );
    }

    #[test]
    fn codex_turn_activity_tracks_running_turn() {
        let activities = codex_turn_activity_from_log_rows([(
            42,
            "thread_a".to_string(),
            "turn{otel.name=\"session_task.turn\" thread.id=thread_a turn.id=turn_a}:run_sampling_request{turn_id=turn_a cwd=/tmp/project}".to_string(),
        )]);

        let activity = activities.get("thread_a:turn_a").unwrap();
        assert_eq!(activity.thread_id, "thread_a");
        assert_eq!(activity.turn_id, "turn_a");
        assert_eq!(activity.cwd.as_deref(), Some("/tmp/project"));
        assert!(!activity.completed);
    }

    #[test]
    fn codex_response_completed_marks_turn_done() {
        let activities = codex_turn_activity_from_log_rows([
            (
                42,
                "thread_a".to_string(),
                "turn{otel.name=\"session_task.turn\" thread.id=thread_a turn.id=turn_a}:receiving_stream".to_string(),
            ),
            (
                43,
                "thread_a".to_string(),
                "turn{otel.name=\"session_task.turn\" thread.id=thread_a turn.id=turn_a}: event.kind=response.completed".to_string(),
            ),
        ]);

        assert!(activities.get("thread_a:turn_a").unwrap().completed);
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
                .find(|session| session.id == activity.session_id);
            let message = latest_messages.get(&activity.session_id);
            let cwd = session
                .map(|session| session.directory.clone())
                .unwrap_or_else(|| String::from(""));
            let name = message
                .and_then(|message| message.prompt.clone())
                .or_else(|| {
                    session
                        .map(|session| session.title.clone())
                        .filter(|title| !title.is_empty() && title != "new session")
                })
                .or_else(|| {
                    cwd.is_empty()
                        .then(|| activity.session_id.clone())
                        .or_else(|| Some(project_name(&cwd)))
                })
                .unwrap_or_else(|| activity.session_id.clone());
            let session_updated_at = session
                .map(|session| session.time_updated)
                .unwrap_or(activity.updated_at);

            Some(Task {
                name,
                pid: 0,
                status: TaskStatus::Running,
                start_time: message
                    .map(|message| message.time_created)
                    .unwrap_or(session_updated_at),
                command: format!("opencode session {}", activity.session_id),
                tool: "OpenCode".to_string(),
                cwd,
                session_id: format!(
                    "opencode:{}:{}",
                    activity.session_id,
                    message
                        .map(|message| message.id.as_str())
                        .unwrap_or("prompt")
                ),
                updated_at: message
                    .map(|message| message.time_updated)
                    .unwrap_or(activity.updated_at)
                    .max(session_updated_at),
            })
        })
        .collect()
}

#[tauri::command]
pub fn get_running_tasks() -> Vec<Task> {
    let mut tasks = claude_tasks(false);
    tasks.extend(opencode_tasks());
    tasks.extend(codex_tasks());
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
