use super::PlatformPort;
use sysinfo::{System, ProcessStatus};

#[allow(dead_code)]
pub struct MacPlatformPort {
    system: System,
}

#[allow(dead_code)]
impl MacPlatformPort {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }
}

impl PlatformPort for MacPlatformPort {
    fn get_process_list(&self) -> Vec<String> {
        self.system
            .processes()
            .iter()
            .map(|(pid, process)| {
                format!(
                    "{}:{}:{}",
                    pid.as_u32(),
                    process.name().to_string_lossy(),
                    matches!(process.status(), ProcessStatus::Run)
                )
            })
            .collect()
    }

    fn is_platform_supported() -> bool {
        true
    }
}
