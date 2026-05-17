#[cfg(target_os = "macos")]
pub mod mac_port;

#[allow(dead_code)]
pub trait PlatformPort {
    fn get_process_list(&self) -> Vec<String>;
    fn is_platform_supported() -> bool;
}

#[cfg(not(target_os = "macos"))]
pub struct DefaultPlatformPort;

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
impl PlatformPort for DefaultPlatformPort {
    fn get_process_list(&self) -> Vec<String> {
        Vec::new()
    }

    fn is_platform_supported() -> bool {
        false
    }
}
