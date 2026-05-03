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
struct Rng {
    state: u64,
}
impl Rng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let seed = seed.wrapping_mul(6364136223846793005);
        Rng {
            state: if seed == 0 { 1 } else { seed },
        }
    }
    #[allow(unused)]
    fn new_seed(seed: u64) -> Self {
        if seed == 0 {
            panic!("Cannot use 1 to random seed!")
        }
        Rng { state: seed }
    }
    // 使劲进行位运算以确保真实随机！
    fn next_u64(&mut self) -> u64 {
        let old_state = self.state;
        self.state = old_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let xor_shifted = ((old_state >> 18) ^ old_state) >> 27;
        let rot = (old_state >> 59) as u32;
        (xor_shifted >> rot) | (xor_shifted.wrapping_shl(64 - rot))
    }
    fn gen_range(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        self.next_u64() % max
    }
}
pub fn gen_random_uuid() -> String {
    let mut rng = Rng::new();
    let p1 = rng.gen_range(u16::MAX as u64);
    let p2 = rng.gen_range(u16::MAX as u64);
    let p3 = rng.gen_range(u16::MAX as u64);
    let p4 = rng.gen_range(u16::MAX as u64);
    let p5 = rng.gen_range(u16::MAX as u64);
    let p6 = rng.gen_range(u16::MAX as u64);
    let p7 = rng.gen_range(u16::MAX as u64);
    let p8 = rng.gen_range(u16::MAX as u64);
    format!(
        "{:04x}{:04x}-{:04x}-{:04x}-{:04x}-{:04x}{:04x}{:04x}",
        p1, p2, p3, p4, p5, p6, p7, p8
    )
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
