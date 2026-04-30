use std::sync::OnceLock;
use tauri::Manager;
pub static HOME_DIR: OnceLock<String> = OnceLock::new();
fn set_file(path: String, contents: &str) -> Option<()> {
    std::fs::write(path, contents.as_bytes()).ok()
}
fn get_file(path: String) -> Option<String> {
    std::fs::read_to_string(path).ok()
}
pub fn init_home_dir(
    app_handle: &tauri::AppHandle,
    dir_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|_| "Cannot get data dir!".to_string())?;
    let home_dir = base_dir.join(dir_name);
    HOME_DIR
        .set(home_dir.to_string_lossy().to_string())
        .map_err(|_| "Cannot put all value!".to_string())?;
    create_dir(HOME_DIR.get().unwrap());
    Ok(())
}
pub fn create_dir(path: &str) -> bool {
    let p = std::path::Path::new(path);
    if !p.exists() || !p.is_dir() {
        if let Err(_) = std::fs::create_dir_all(p) {
            return false;
        }
    }
    return true;
}
#[cfg(all(target_os = "macos", not(debug_assertions)))]
pub fn get_executable_file_path() -> Option<String> {
    Some(
        std::env::current_exe()
            .ok()?
            .parent()?
            .parent()?
            .parent()?
            .parent()?
            .to_string_lossy()
            .to_string(),
    )
}
#[cfg(any(not(target_os = "macos"), debug_assertions))]
pub fn get_executable_file_path() -> Option<String> {
    Some(
        std::env::current_exe()
            .ok()?
            .parent()?
            .to_string_lossy()
            .to_string(),
    )
}
#[macro_export]
macro_rules! path_join {
    ($($part:expr),*) => {{
        let mut path_buf = std::path::PathBuf::new();
        $(path_buf.push($part);)*
        path_buf.to_string_lossy().to_string()
    }};
}
