use serde::{Deserialize, Serialize};
use sysinfo::{System, ProcessStatus};
use std::sync::Mutex;

static SYSTEM: Mutex<Option<System>> = Mutex::new(None);

#[allow(dead_code)]
pub fn init_system() {
    let mut sys = SYSTEM.lock().unwrap();
    *sys = Some(System::new_all());
}

#[allow(dead_code)]
pub fn with_system<F, R>(f: F) -> R
where
    F: FnOnce(&System) -> R {
    let sys = SYSTEM.lock().unwrap();
    match &*sys {
        Some(s) => f(s),
        None => {
            drop(sys);
            let new_sys = System::new_all();
            let result = f(&new_sys);
            let mut lock = SYSTEM.lock().unwrap();
            *lock = Some(new_sys);
            result
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub pid: u32,
    pub status: TaskStatus,
    pub start_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Running,
    Stopped,
    Unknown,
}

const MONITORED_PROCESSES: &[&str] = &[
    "claude",
    "cursor",
    "trae",
    "windsurf",
    "warp",
    "opencode",
    "code",
];

#[allow(dead_code)]
pub fn get_monitored_processes() -> Vec<&'static str> {
    MONITORED_PROCESSES.iter().copied().collect()
}

fn is_process_running(status: ProcessStatus) -> bool {
    matches!(status, ProcessStatus::Run)
}

#[tauri::command]
pub fn get_running_tasks() -> Vec<Task> {
    with_system(|sys| {
        sys.processes()
            .iter()
            .filter(|(_, process)| {
                let name = process.name().to_string_lossy();
                MONITORED_PROCESSES.iter().any(|&p| name.contains(p))
            })
            .map(|(pid, process)| Task {
                name: process.name().to_string_lossy().to_string(),
                pid: pid.as_u32(),
                status: if is_process_running(process.status()) {
                    TaskStatus::Running
                } else {
                    TaskStatus::Unknown
                },
                start_time: process.start_time(),
            })
            .collect()
    })
}

#[tauri::command]
pub fn get_task_count() -> usize {
    with_system(|sys| {
        sys.processes()
            .iter()
            .filter(|(_, process)| {
                let name = process.name().to_string_lossy();
                MONITORED_PROCESSES.iter().any(|&p| name.contains(p))
            })
            .count()
    })
}

#[allow(dead_code)]
pub fn check_tasks_changed(last_tasks: &[String], current_tasks: &[Task]) -> bool {
    let last_names: Vec<String> = last_tasks.to_vec();
    let current_names: Vec<String> = current_tasks.iter().map(|t| t.name.clone()).collect();

    last_names != current_names
}
