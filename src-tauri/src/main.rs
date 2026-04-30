// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod aeskey;
mod decrypt;
mod lua_reader;
mod parser;
mod util;
use aeskey::AES_KEY;
use decrypt::decrypt_to_memory;
use parser::parse_embed;
use util::*;

#[derive(serde::Deserialize, serde::Serialize)]
enum ErrorCode {
    CannotGetExecuteDir = -1,
    CannotReadGamesPath = -2,
    CannotCreateGameDir = -3,
    CannotDecryptData = -4,
    CannotInitHomeDir = -10000,
    AesKeyHasWrong = -2147483647,
    NotImplements = -2147483648,
}
#[derive(serde::Deserialize, serde::Serialize)]
pub struct CopywritingStruct {
    // 全局定义
    define: serde_json::Map<String, serde_json::Value>,
    // 部分二进制资源。直接使用 Vec<u8> 返回原始数据。。。
    resource: serde_json::Map<String, serde_json::Value>,
    // 当前语言文件里面的总数
    locale: serde_json::Map<String, serde_json::Value>,
    // 当前翻译
    translate: serde_json::Map<String, serde_json::Value>,
    // 当前样式表
    style: serde_json::Map<String, serde_json::Value>,
    // 最终的文案代码！
    copywriting: serde_json::Value,
}
pub static COPY_WRITING: std::sync::OnceLock<CopywritingStruct> = std::sync::OnceLock::new();

#[tauri::command]
fn init_copywriting(
    app_handle: tauri::AppHandle,
    file_name: String,
) -> Result<CopywritingStruct, ErrorCode> {
    use base64::Engine;
    // init_home_dir(&app_handle, format!("{}", file_name.as_str()).as_str()).map_err(|_| ErrorCode::CannotInitHomeDir)?;
    let execute_dir = get_executable_file_path().ok_or(ErrorCode::CannotInitHomeDir)?;
    let mut engine_path = std::path::PathBuf::new();
    engine_path.push(execute_dir);
    engine_path.push("games");
    engine_path.push(file_name);
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(AES_KEY)
        .map_err(|_| ErrorCode::AesKeyHasWrong)?;
    if key_bytes.len() != 32 {
        return Err(ErrorCode::AesKeyHasWrong);
    }
    let key_slice: [u8; 32] = key_bytes.try_into().unwrap();
    let engine_path = engine_path.to_string_lossy().to_string();
    let (file_map, main_lua_file_name) =
        decrypt_to_memory(engine_path, key_slice).map_err(|_| ErrorCode::CannotDecryptData)?;
    let Some(main_lua_file_name) = main_lua_file_name else {
        return Err(ErrorCode::CannotDecryptData);
    };
    let main_lua_content = String::from_utf8(file_map[&main_lua_file_name].clone())
        .map_err(|_| ErrorCode::CannotDecryptData)?;
    let real_main_lua_content = parse_embed(
        main_lua_file_name.as_str(),
        main_lua_content.as_str(),
        &file_map.clone(),
    );
    println!("{}", real_main_lua_content);
    Err(ErrorCode::NotImplements)
}
///
/// 遍历文件夹，找到所有 .rrs 的文件名。
///
#[tauri::command]
fn find_all_game_file_name() -> Result<Vec<String>, ErrorCode> {
    let execute_dir = get_executable_file_path().ok_or(ErrorCode::CannotGetExecuteDir)?;
    let mut engine_path = std::path::PathBuf::new();
    engine_path.push(execute_dir);
    engine_path.push("games");
    if !create_dir(engine_path.to_string_lossy().to_string().as_str()) {
        return Err(ErrorCode::CannotCreateGameDir);
    }
    let file_entry = std::fs::read_dir(engine_path).map_err(|_| ErrorCode::CannotReadGamesPath)?;
    let mut result: Vec<String> = Vec::new();
    for entry in file_entry {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let Some(ext) = path.extension() else {
            continue;
        };
        if path.is_file() && ext == "rrs" {
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let file_name = file_name.to_string_lossy().to_string();
            result.push(file_name);
        } else {
            continue;
        }
    }
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            init_copywriting,
            find_all_game_file_name
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
